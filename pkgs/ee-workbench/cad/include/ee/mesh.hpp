#pragma once

#include <string>
#include <vector>

class TopoDS_Shape;

namespace ee {

/// Tessellation is the only lossy step between the solid and the printer, so
/// both deviations stay explicit instead of hiding in a preferences group.
struct Tessellation
{
    double deflection = 0.1;
    double angular = 0.5;
};

struct MeshStats
{
    long long triangles = 0;
    long long bytes = 0;
};

struct Vec3
{
    double x = 0.0;
    double y = 0.0;
    double z = 0.0;
};

/// One tessellated triangle in global millimetres, wound counter-clockwise
/// seen from outside. `normal` is the unit face normal, or zero when the
/// triangle is degenerate.
struct Facet
{
    Vec3 vertex[3];
    Vec3 normal;
};

/// Meshes `shape` and returns its triangles. Both the STL export and the
/// offscreen render read the model through this, so what a viewer opens and
/// what a render shows can never disagree about the geometry.
std::vector<Facet> tessellate(const TopoDS_Shape& shape, const Tessellation& tessellation);

/// Tessellates `shape` and replaces `path` with a binary STL of it. The write
/// goes to a sibling temporary and is renamed, so a viewer watching the path
/// either sees the previous mesh or the new one, never a truncated file.
MeshStats write_binary_stl(const TopoDS_Shape& shape,
                           const std::string& path,
                           const Tessellation& tessellation);

/// Replaces `path` with the bytes, through the same temporary-then-rename.
void write_atomically(const std::string& path, const std::string& bytes);

}  // namespace ee
