#pragma once

#include <string>

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

/// Tessellates `shape` and replaces `path` with a binary STL of it. The write
/// goes to a sibling temporary and is renamed, so a viewer watching the path
/// either sees the previous mesh or the new one, never a truncated file.
MeshStats write_binary_stl(const TopoDS_Shape& shape,
                           const std::string& path,
                           const Tessellation& tessellation);

}  // namespace ee
