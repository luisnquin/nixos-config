#include "ee/mesh.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <stdexcept>
#include <system_error>
#include <utility>
#include <vector>

#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <Poly_Triangulation.hxx>
#include <TopAbs_Orientation.hxx>
#include <TopExp_Explorer.hxx>
#include <TopLoc_Location.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>
#include <gp.hxx>
#include <gp_Pnt.hxx>
#include <gp_Vec.hxx>

namespace ee {
namespace {

static_assert(sizeof(float) == 4, "binary STL stores IEEE-754 single precision");

void put_u32(std::string& out, std::uint32_t value)
{
    for (int shift = 0; shift < 32; shift += 8) {
        out.push_back(static_cast<char>((value >> shift) & 0xFF));
    }
}

void put_float(std::string& out, double value)
{
    const float narrowed = static_cast<float>(value);
    std::uint32_t bits = 0;
    std::memcpy(&bits, &narrowed, sizeof(bits));
    put_u32(out, bits);
}

void put_point(std::string& out, const gp_Pnt& point)
{
    put_float(out, point.X());
    put_float(out, point.Y());
    put_float(out, point.Z());
}

}  // namespace

MeshStats write_binary_stl(const TopoDS_Shape& shape,
                           const std::string& path,
                           const Tessellation& tessellation)
{
    if (shape.IsNull()) {
        throw std::runtime_error("shape is empty");
    }
    if (!(tessellation.deflection > 0.0) || !(tessellation.angular > 0.0)) {
        throw std::runtime_error("deflection and angular deviation must be positive");
    }

    // Cleaning first makes the export reproducible: a shape carries whatever
    // triangulation the viewport asked for last.
    BRepMesh_IncrementalMesh mesher(shape,
                                    tessellation.deflection,
                                    Standard_False,
                                    tessellation.angular,
                                    Standard_True);
    mesher.Perform();

    std::string body;
    std::uint32_t triangles = 0;

    for (TopExp_Explorer faces(shape, TopAbs_FACE); faces.More(); faces.Next()) {
        const TopoDS_Face& face = TopoDS::Face(faces.Current());
        TopLoc_Location location;
        const Handle(Poly_Triangulation) mesh = BRep_Tool::Triangulation(face, location);
        if (mesh.IsNull()) {
            continue;
        }

        const gp_Trsf& placement = location.Transformation();
        // A reversed face keeps its triangles wound for the forward face, so
        // the winding has to be flipped or the printed normals point inwards.
        const bool reversed = face.Orientation() == TopAbs_REVERSED;

        for (Standard_Integer index = 1; index <= mesh->NbTriangles(); ++index) {
            Standard_Integer a = 0;
            Standard_Integer b = 0;
            Standard_Integer c = 0;
            mesh->Triangle(index).Get(a, b, c);
            if (reversed) {
                std::swap(b, c);
            }

            const gp_Pnt first = mesh->Node(a).Transformed(placement);
            const gp_Pnt second = mesh->Node(b).Transformed(placement);
            const gp_Pnt third = mesh->Node(c).Transformed(placement);

            gp_Vec normal = gp_Vec(first, second).Crossed(gp_Vec(first, third));
            const double magnitude = normal.Magnitude();
            if (magnitude > gp::Resolution()) {
                normal.Divide(magnitude);
            }
            else {
                normal = gp_Vec(0.0, 0.0, 0.0);
            }

            put_float(body, normal.X());
            put_float(body, normal.Y());
            put_float(body, normal.Z());
            put_point(body, first);
            put_point(body, second);
            put_point(body, third);
            body.push_back('\0');
            body.push_back('\0');
            ++triangles;
        }
    }

    if (triangles == 0) {
        throw std::runtime_error("tessellation produced no triangles");
    }

    std::string out(80, '\0');
    const std::string banner = "ee-workbench binary STL";
    std::memcpy(out.data(), banner.data(), banner.size());
    put_u32(out, triangles);
    out += body;

    const std::string temporary = path + ".partial";
    {
        std::ofstream file(temporary, std::ios::binary | std::ios::trunc);
        if (!file) {
            throw std::system_error(errno, std::generic_category(), "open " + temporary);
        }
        file.write(out.data(), static_cast<std::streamsize>(out.size()));
        file.close();
        if (!file) {
            throw std::system_error(errno, std::generic_category(), "write " + temporary);
        }
    }
    if (std::rename(temporary.c_str(), path.c_str()) != 0) {
        const int failure = errno;
        std::remove(temporary.c_str());
        throw std::system_error(failure, std::generic_category(), "rename onto " + path);
    }

    return MeshStats{static_cast<long long>(triangles), static_cast<long long>(out.size())};
}

}  // namespace ee
