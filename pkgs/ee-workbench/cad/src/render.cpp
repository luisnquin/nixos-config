#include "ee/render.hpp"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

#include <zlib.h>

namespace ee {
namespace {

/// Two samples per pixel per axis. The whole point of the render is that a
/// reader can tell a chamfer from a step, and a jagged silhouette hides both.
constexpr int kSupersample = 2;
/// Pixels of blank frame around the fitted model, in final pixels.
constexpr int kMargin = 24;
/// Faces meeting at more than this are drawn as an edge. 25 degrees keeps a
/// tessellated cylinder smooth while every box corner still shows.
constexpr double kCreaseCos = 0.906;
constexpr int kTriadPixels = 44;

struct Rgb
{
    double r = 0.0;
    double g = 0.0;
    double b = 0.0;
};

const Rgb kBackground{0.949, 0.949, 0.961};
const Rgb kSurface{0.553, 0.600, 0.655};
const Rgb kEdge{0.106, 0.122, 0.141};
const Rgb kAxisX{0.804, 0.243, 0.243};
const Rgb kAxisY{0.243, 0.643, 0.325};
const Rgb kAxisZ{0.239, 0.451, 0.816};

double dot(const Vec3& a, const Vec3& b)
{
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

Vec3 cross(const Vec3& a, const Vec3& b)
{
    return Vec3{a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x};
}

Vec3 scale(const Vec3& a, double factor)
{
    return Vec3{a.x * factor, a.y * factor, a.z * factor};
}

Vec3 add(const Vec3& a, const Vec3& b)
{
    return Vec3{a.x + b.x, a.y + b.y, a.z + b.z};
}

Vec3 unit(const Vec3& a)
{
    const double length = std::sqrt(dot(a, a));
    return length > 1e-12 ? scale(a, 1.0 / length) : Vec3{};
}

/// Screen basis from a look direction: right first, then up, so a view is
/// named by where the camera stands and never by a hand-written matrix.
View basis_for(const Vec3& forward)
{
    const Vec3 f = unit(forward);
    Vec3 world_up{0.0, 0.0, 1.0};
    if (std::abs(dot(f, world_up)) > 0.99) {
        world_up = Vec3{0.0, 1.0, 0.0};
    }

    View out;
    out.forward = f;
    out.right = unit(cross(f, world_up));
    out.up = unit(cross(out.right, f));
    return out;
}

void put_u32(std::string& out, std::uint32_t value)
{
    for (int shift = 24; shift >= 0; shift -= 8) {
        out.push_back(static_cast<char>((value >> shift) & 0xFF));
    }
}

void put_chunk(std::string& out, const char* type, const std::string& payload)
{
    put_u32(out, static_cast<std::uint32_t>(payload.size()));
    const std::size_t start = out.size();
    out.append(type, 4);
    out += payload;
    const auto* bytes = reinterpret_cast<const Bytef*>(out.data() + start);
    put_u32(out,
            static_cast<std::uint32_t>(
                ::crc32(0, bytes, static_cast<uInt>(out.size() - start))));
}

std::uint8_t quantize(double value)
{
    const double clamped = std::clamp(value, 0.0, 1.0);
    return static_cast<std::uint8_t>(std::lround(clamped * 255.0));
}

std::string encode_png(const std::vector<Rgb>& pixels, int width, int height)
{
    std::string raw;
    raw.reserve(static_cast<std::size_t>(height) * (1 + 3 * static_cast<std::size_t>(width)));
    for (int y = 0; y < height; ++y) {
        raw.push_back('\0');  // filter: none, so the image stays trivially readable
        for (int x = 0; x < width; ++x) {
            const Rgb& pixel = pixels[static_cast<std::size_t>(y) * width + x];
            raw.push_back(static_cast<char>(quantize(pixel.r)));
            raw.push_back(static_cast<char>(quantize(pixel.g)));
            raw.push_back(static_cast<char>(quantize(pixel.b)));
        }
    }

    uLongf packed_size = ::compressBound(static_cast<uLong>(raw.size()));
    std::string packed(packed_size, '\0');
    const int packed_ok = ::compress2(reinterpret_cast<Bytef*>(packed.data()),
                                      &packed_size,
                                      reinterpret_cast<const Bytef*>(raw.data()),
                                      static_cast<uLong>(raw.size()),
                                      Z_DEFAULT_COMPRESSION);
    if (packed_ok != Z_OK) {
        throw std::runtime_error("zlib refused to compress the image");
    }
    packed.resize(packed_size);

    std::string header;
    put_u32(header, static_cast<std::uint32_t>(width));
    put_u32(header, static_cast<std::uint32_t>(height));
    header.push_back(8);  // bit depth
    header.push_back(2);  // colour type: truecolour
    header.push_back(0);  // deflate
    header.push_back(0);  // adaptive filtering
    header.push_back(0);  // no interlace

    std::string out("\x89PNG\r\n\x1a\n", 8);
    put_chunk(out, "IHDR", header);
    put_chunk(out, "IDAT", packed);
    put_chunk(out, "IEND", std::string());
    return out;
}

struct Canvas
{
    int width = 0;
    int height = 0;
    std::vector<Rgb> colour;
    std::vector<double> depth;
    std::vector<Vec3> normal;

    void reset(int w, int h)
    {
        width = w;
        height = h;
        const std::size_t count = static_cast<std::size_t>(w) * h;
        colour.assign(count, kBackground);
        depth.assign(count, std::numeric_limits<double>::infinity());
        normal.assign(count, Vec3{});
    }

    bool covered(int x, int y) const
    {
        return std::isfinite(depth[static_cast<std::size_t>(y) * width + x]);
    }
};

/// A vertex already in screen space: x and y in pixels, z in millimetres along
/// the view direction.
struct Projected
{
    double x = 0.0;
    double y = 0.0;
    double z = 0.0;
};

void raster_triangle(Canvas& canvas,
                     const Projected& a,
                     const Projected& b,
                     const Projected& c,
                     const Vec3& normal,
                     const Rgb& shade)
{
    const double area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
    if (std::abs(area) < 1e-9) {
        return;
    }

    const int min_x = std::max(0, static_cast<int>(std::floor(std::min({a.x, b.x, c.x}))));
    const int max_x = std::min(canvas.width - 1,
                               static_cast<int>(std::ceil(std::max({a.x, b.x, c.x}))));
    const int min_y = std::max(0, static_cast<int>(std::floor(std::min({a.y, b.y, c.y}))));
    const int max_y = std::min(canvas.height - 1,
                               static_cast<int>(std::ceil(std::max({a.y, b.y, c.y}))));

    for (int y = min_y; y <= max_y; ++y) {
        for (int x = min_x; x <= max_x; ++x) {
            const double px = x + 0.5;
            const double py = y + 0.5;

            const double w0 = ((b.x - px) * (c.y - py) - (c.x - px) * (b.y - py)) / area;
            const double w1 = ((c.x - px) * (a.y - py) - (a.x - px) * (c.y - py)) / area;
            const double w2 = 1.0 - w0 - w1;
            if (w0 < 0.0 || w1 < 0.0 || w2 < 0.0) {
                continue;
            }

            const double z = w0 * a.z + w1 * b.z + w2 * c.z;
            const std::size_t at = static_cast<std::size_t>(y) * canvas.width + x;
            if (z >= canvas.depth[at]) {
                continue;
            }
            canvas.depth[at] = z;
            canvas.colour[at] = shade;
            canvas.normal[at] = normal;
        }
    }
}

/// Silhouettes and creases, read back off the depth and normal buffers. A flat
/// shaded box without them is one grey blob; with them it is a box.
/// Silhouettes, creases and depth cliffs. The last one is what makes a pocket
/// visible: its floor and the face around it share a normal and a colour, so
/// without a depth test a hole reads as flat material.
void draw_edges(Canvas& canvas, double depth_step)
{
    std::vector<Rgb> out = canvas.colour;

    auto depth_at = [&canvas](int x, int y) {
        return canvas.depth[static_cast<std::size_t>(y) * canvas.width + x];
    };

    for (int y = 0; y < canvas.height; ++y) {
        for (int x = 0; x < canvas.width; ++x) {
            if (!canvas.covered(x, y)) {
                continue;
            }
            const std::size_t at = static_cast<std::size_t>(y) * canvas.width + x;

            bool edge = false;
            const int neighbours[4][2] = {{1, 0}, {-1, 0}, {0, 1}, {0, -1}};
            for (const auto& step : neighbours) {
                const int nx = x + step[0];
                const int ny = y + step[1];
                if (nx < 0 || ny < 0 || nx >= canvas.width || ny >= canvas.height) {
                    edge = true;
                    break;
                }
                if (!canvas.covered(nx, ny)) {
                    edge = true;
                    break;
                }
                const std::size_t other = static_cast<std::size_t>(ny) * canvas.width + nx;
                if (dot(canvas.normal[at], canvas.normal[other]) < kCreaseCos) {
                    edge = true;
                    break;
                }
            }

            // A second difference rather than a first: a slanted plane has a
            // large depth gradient and no curvature, so only a genuine step
            // survives this.
            const int axes[2][2] = {{1, 0}, {0, 1}};
            for (const auto& step : axes) {
                if (edge) {
                    break;
                }
                const int px = x - step[0];
                const int py = y - step[1];
                const int nx = x + step[0];
                const int ny = y + step[1];
                if (px < 0 || py < 0 || nx >= canvas.width || ny >= canvas.height) {
                    continue;
                }
                if (!canvas.covered(px, py) || !canvas.covered(nx, ny)) {
                    continue;
                }
                const double curvature =
                    std::abs(depth_at(px, py) + depth_at(nx, ny) - 2.0 * depth_at(x, y));
                if (curvature > depth_step) {
                    edge = true;
                }
            }

            if (edge) {
                out[at] = kEdge;
            }
        }
    }

    canvas.colour = std::move(out);
}

void stamp(Canvas& canvas, double x, double y, double radius, const Rgb& colour)
{
    const int min_x = std::max(0, static_cast<int>(std::floor(x - radius)));
    const int max_x = std::min(canvas.width - 1, static_cast<int>(std::ceil(x + radius)));
    const int min_y = std::max(0, static_cast<int>(std::floor(y - radius)));
    const int max_y = std::min(canvas.height - 1, static_cast<int>(std::ceil(y + radius)));

    for (int py = min_y; py <= max_y; ++py) {
        for (int px = min_x; px <= max_x; ++px) {
            const double dx = px + 0.5 - x;
            const double dy = py + 0.5 - y;
            if (dx * dx + dy * dy > radius * radius) {
                continue;
            }
            canvas.colour[static_cast<std::size_t>(py) * canvas.width + px] = colour;
        }
    }
}

void draw_line(Canvas& canvas,
               double x0,
               double y0,
               double x1,
               double y1,
               double radius,
               const Rgb& colour)
{
    const double steps = std::max(std::abs(x1 - x0), std::abs(y1 - y0));
    const int count = std::max(1, static_cast<int>(std::ceil(steps)));
    for (int step = 0; step <= count; ++step) {
        const double t = static_cast<double>(step) / count;
        stamp(canvas, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, radius, colour);
    }
}

/// The corner triad. Without it an isometric view is unreadable: the reader
/// can see a shape but cannot say which way it is pointing.
void draw_triad(Canvas& canvas, const View& view, int supersample)
{
    const double length = kTriadPixels * supersample;
    const double origin_x = length + 12.0 * supersample;
    const double origin_y = canvas.height - length - 12.0 * supersample;

    struct Axis
    {
        Vec3 direction;
        Rgb colour;
    };
    Axis axes[3] = {{Vec3{1.0, 0.0, 0.0}, kAxisX},
                    {Vec3{0.0, 1.0, 0.0}, kAxisY},
                    {Vec3{0.0, 0.0, 1.0}, kAxisZ}};

    // Farthest first, so the axis pointing away from the reader is the one
    // that gets covered where two overlap.
    std::sort(std::begin(axes), std::end(axes), [&view](const Axis& a, const Axis& b) {
        return dot(a.direction, view.forward) > dot(b.direction, view.forward);
    });

    for (const Axis& axis : axes) {
        const double dx = dot(axis.direction, view.right) * length;
        const double dy = -dot(axis.direction, view.up) * length;
        draw_line(canvas,
                  origin_x,
                  origin_y,
                  origin_x + dx,
                  origin_y + dy,
                  1.6 * supersample,
                  axis.colour);
        stamp(canvas, origin_x + dx, origin_y + dy, 3.0 * supersample, axis.colour);
    }
}

}  // namespace

bool view_named(const std::string& name, View& out)
{
    if (name == "iso") {
        // The camera stands at +x, -y, +z: the corner FreeCAD calls axonometric.
        out = basis_for(Vec3{-1.0, 1.0, -1.0});
    }
    else if (name == "front") {
        out = basis_for(Vec3{0.0, 1.0, 0.0});
    }
    else if (name == "back") {
        out = basis_for(Vec3{0.0, -1.0, 0.0});
    }
    else if (name == "left") {
        out = basis_for(Vec3{1.0, 0.0, 0.0});
    }
    else if (name == "right") {
        out = basis_for(Vec3{-1.0, 0.0, 0.0});
    }
    else if (name == "top") {
        out = basis_for(Vec3{0.0, 0.0, -1.0});
    }
    else if (name == "bottom") {
        out = basis_for(Vec3{0.0, 0.0, 1.0});
    }
    else {
        return false;
    }
    return true;
}

RenderStats write_png(const std::vector<Facet>& facets,
                      const std::string& path,
                      const RenderRequest& request)
{
    if (facets.empty()) {
        throw std::runtime_error("nothing to render");
    }
    if (request.width < 64 || request.height < 64 || request.width > 4096 ||
        request.height > 4096) {
        throw std::runtime_error("width and height must be between 64 and 4096 pixels");
    }

    View view;
    if (!view_named(request.view, view)) {
        throw std::runtime_error("unknown view " + request.view);
    }

    double min_u = std::numeric_limits<double>::infinity();
    double max_u = -min_u;
    double min_v = min_u;
    double max_v = -min_u;
    double min_w = min_u;
    double max_w = -min_u;
    for (const Facet& facet : facets) {
        for (const Vec3& vertex : facet.vertex) {
            const double u = dot(vertex, view.right);
            const double v = dot(vertex, view.up);
            const double w = dot(vertex, view.forward);
            min_u = std::min(min_u, u);
            max_u = std::max(max_u, u);
            min_v = std::min(min_v, v);
            max_v = std::max(max_v, v);
            min_w = std::min(min_w, w);
            max_w = std::max(max_w, w);
        }
    }

    const int width = request.width;
    const int height = request.height;
    const int big_width = width * kSupersample;
    const int big_height = height * kSupersample;
    const double margin = kMargin * kSupersample;

    // A plate seen edge-on has no extent in one axis; a floor keeps the fit
    // finite instead of dividing by zero.
    const double span_u = std::max(max_u - min_u, 1e-6);
    const double span_v = std::max(max_v - min_v, 1e-6);
    const double pixels_per_mm = std::min((big_width - 2.0 * margin) / span_u,
                                          (big_height - 2.0 * margin) / span_v);
    const double centre_u = 0.5 * (min_u + max_u);
    const double centre_v = 0.5 * (min_v + max_v);

    Canvas canvas;
    canvas.reset(big_width, big_height);

    const Vec3 key = unit(add(add(scale(view.right, -0.45), scale(view.up, 0.55)),
                              scale(view.forward, -1.0)));
    const Vec3 fill = unit(add(add(scale(view.right, 0.70), scale(view.up, -0.25)),
                               scale(view.forward, -1.0)));

    for (const Facet& facet : facets) {
        const Vec3& normal = facet.normal;
        if (dot(normal, normal) < 0.5) {
            continue;
        }

        const double light = 0.24 + 0.62 * std::max(0.0, dot(normal, key)) +
                             0.16 * std::max(0.0, dot(normal, fill));
        const Rgb shade{kSurface.r * light, kSurface.g * light, kSurface.b * light};

        Projected corner[3];
        for (int index = 0; index < 3; ++index) {
            const Vec3& vertex = facet.vertex[index];
            corner[index].x =
                big_width * 0.5 + (dot(vertex, view.right) - centre_u) * pixels_per_mm;
            corner[index].y =
                big_height * 0.5 - (dot(vertex, view.up) - centre_v) * pixels_per_mm;
            corner[index].z = dot(vertex, view.forward);
        }
        raster_triangle(canvas, corner[0], corner[1], corner[2], normal, shade);
    }

    // Half a percent of the model's diagonal: coarse enough that a tessellated
    // cylinder's own curvature stays under it, fine enough that any pocket deep
    // enough to matter draws a rim.
    const double diagonal = std::sqrt(span_u * span_u + span_v * span_v +
                                      (max_w - min_w) * (max_w - min_w));
    draw_edges(canvas, std::max(0.005 * diagonal, 1e-6));
    draw_triad(canvas, view, kSupersample);

    std::vector<Rgb> pixels(static_cast<std::size_t>(width) * height);
    const double weight = 1.0 / (kSupersample * kSupersample);
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            Rgb sum{};
            for (int sy = 0; sy < kSupersample; ++sy) {
                for (int sx = 0; sx < kSupersample; ++sx) {
                    const std::size_t at =
                        static_cast<std::size_t>(y * kSupersample + sy) * big_width +
                        (x * kSupersample + sx);
                    sum.r += canvas.colour[at].r;
                    sum.g += canvas.colour[at].g;
                    sum.b += canvas.colour[at].b;
                }
            }
            pixels[static_cast<std::size_t>(y) * width + x] =
                Rgb{sum.r * weight, sum.g * weight, sum.b * weight};
        }
    }

    const std::string bytes = encode_png(pixels, width, height);
    write_atomically(path, bytes);

    RenderStats stats;
    stats.width = width;
    stats.height = height;
    stats.bytes = static_cast<long long>(bytes.size());
    stats.triangles = static_cast<long long>(facets.size());
    stats.mm_per_pixel = kSupersample / pixels_per_mm;
    stats.view = view;
    return stats;
}

}  // namespace ee
