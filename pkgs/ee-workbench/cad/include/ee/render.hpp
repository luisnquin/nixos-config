#pragma once

#include <string>
#include <vector>

#include "ee/mesh.hpp"

namespace ee {

/// A named orthographic viewpoint. The names are the whole vocabulary: a
/// caller that cannot spell a camera cannot aim one badly.
struct View
{
    Vec3 forward;  ///< the direction the camera looks along, in global axes
    Vec3 right;    ///< global direction of one pixel step to the right
    Vec3 up;       ///< global direction of one pixel step upwards
};

/// Nothing but "iso", "front", "back", "left", "right", "top", "bottom".
/// Returns false for anything else instead of guessing a camera.
bool view_named(const std::string& name, View& out);

struct RenderRequest
{
    std::string view = "iso";
    int width = 900;
    int height = 700;
};

struct RenderStats
{
    int width = 0;
    int height = 0;
    long long bytes = 0;
    long long triangles = 0;
    /// The image is fitted to the model, so a pixel is only meaningful next to
    /// this: the reader can measure the part off the picture.
    double mm_per_pixel = 0.0;
    View view;
};

/// Rasterizes `facets` into a PNG at `path`: orthographic, z-buffered, flat
/// shaded, with creases and silhouettes darkened and an axis triad in the
/// corner. Entirely on the CPU, so it needs no display, no GL context and no
/// FreeCAD GUI - the headless session can answer "what does it look like".
RenderStats write_png(const std::vector<Facet>& facets,
                      const std::string& path,
                      const RenderRequest& request);

}  // namespace ee
