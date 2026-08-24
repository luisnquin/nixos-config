#include "ee/session.hpp"

#include <algorithm>
#include <cmath>
#include <functional>
#include <initializer_list>
#include <map>
#include <numeric>
#include <set>
#include <string>
#include <vector>

#include <App/Application.h>
#include <App/Datums.h>
#include <App/Document.h>
#include <App/DocumentObject.h>
#include <App/Origin.h>
#include <App/PropertyUnits.h>
#include <App/VarSet.h>
#include <Base/Interpreter.h>
#include <Base/Placement.h>
#include <Base/Rotation.h>
#include <Base/Tools.h>
#include <Base/Vector3D.h>
#include <Mod/Part/App/Geometry.h>
#include <Mod/Part/App/Part2DObject.h>
#include <Mod/Part/App/PartFeature.h>
#include <Mod/PartDesign/App/Body.h>
#include <Mod/PartDesign/App/Feature.h>
#include <Mod/PartDesign/App/FeatureBoolean.h>
#include <Mod/PartDesign/App/FeatureChamfer.h>
#include <Mod/PartDesign/App/FeatureDressUp.h>
#include <Mod/PartDesign/App/FeatureExtrude.h>
#include <Mod/PartDesign/App/FeatureFillet.h>
#include <Mod/PartDesign/App/FeatureGroove.h>
#include <Mod/PartDesign/App/FeatureLinearPattern.h>
#include <Mod/PartDesign/App/FeatureLoft.h>
#include <Mod/PartDesign/App/FeatureMirrored.h>
#include <Mod/PartDesign/App/FeaturePad.h>
#include <Mod/PartDesign/App/FeaturePocket.h>
#include <Mod/PartDesign/App/FeaturePolarPattern.h>
#include <Mod/PartDesign/App/FeatureRevolution.h>
#include <Mod/PartDesign/App/FeatureSketchBased.h>
#include <Mod/Sketcher/App/Constraint.h>
#include <Mod/Sketcher/App/GeoEnum.h>
#include <Mod/Sketcher/App/GeometryFacade.h>
#include <Mod/Sketcher/App/SketchObject.h>

#include <gp_Dir.hxx>
#include <gp_Pnt.hxx>

#include <BRepAdaptor_Curve.hxx>
#include <BRepBndLib.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <Bnd_Box.hxx>
#include <GeomAbs_CurveType.hxx>
#include <GProp_GProps.hxx>
#include <Precision.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Edge.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Shape.hxx>
#include <TopoDS_Vertex.hxx>
#include <TopTools_IndexedDataMapOfShapeListOfShape.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>

#include "ee/gui.hpp"
#include "ee/mesh.hpp"
#include "ee/paths.hpp"
#include "ee/protocol.hpp"
#include "ee/render.hpp"

namespace ee {
namespace {

using Sketcher::PointPos;

/// Sketcher marks "no second element" with this sentinel rather than an
/// optional, and repeats it in the saved file.
constexpr int kGeoUndefined = Sketcher::GeoEnum::GeoUndef;
/// The sketch origin point lives on an implicit geometry with index -1.
constexpr int kRootPoint = Sketcher::GeoEnum::RtPnt;

/// How close an edge's own bounding box has to sit to the body's, in mm, to
/// count as "on" that face for `--near-min`/`--near-max`. Looser than
/// `measure()`'s micron display precision on purpose: this compares two boxes
/// computed by different OCCT calls, not the same value read back.
constexpr double kSelectionTolerance = 1e-4;

/// Millimetres are reported to a micron. Anything finer is the arithmetic
/// talking, not the model: OCCT pads a bounding box by `Precision::Confusion()`
/// whatever gap you ask for, and an agent comparing numbers should not have to
/// tell -3 from -3.0000001.
double measure(double value)
{
    const double snapped = std::round(value * 1e6) / 1e6;
    return snapped == 0.0 ? 0.0 : snapped;
}

// `Document::addObject` runs every requested name through this same function
// before using it, silently replacing whatever it rejects — so a hyphenated
// name is accepted, then answers to a different name than the one that was
// typed. Asking it up front, before anything is created, means a bad name is
// refused instead of being turned into an object nobody can address.
void require_document_identifier(const std::string& name)
{
    if (Base::Tools::getIdentifier(name) != name) {
        throw Error{"invalid-name",
                    name + " is not usable as a FreeCAD object name: it must start with a "
                           "letter or underscore, and hold only letters, digits and "
                           "underscores after that"};
    }
}

App::Document& document_for(const std::string& name)
{
    App::Document* doc = name.empty() ? App::GetApplication().getActiveDocument()
                                      : App::GetApplication().getDocument(name.c_str());
    if (doc == nullptr) {
        throw Error{"unknown-document",
                    name.empty() ? "no active document" : "no document named " + name};
    }
    return *doc;
}

/// Resolves an object by name, or by being the only one of its type when the
/// caller did not name one. Anything ambiguous is a refusal, never a guess.
template<class T>
T& object_for(App::Document& doc, const std::string& name, const char* kind)
{
    if (!name.empty()) {
        App::DocumentObject* object = doc.getObject(name.c_str());
        if (object == nullptr || !object->isDerivedFrom(T::getClassTypeId())) {
            throw Error{std::string("unknown-") + kind,
                        std::string("no ") + kind + " named " + name + " in " + doc.getName()};
        }
        return *static_cast<T*>(object);
    }

    std::vector<T*> found = doc.getObjectsOfType<T>();
    if (found.empty()) {
        throw Error{std::string("unknown-") + kind,
                    std::string("document ") + doc.getName() + " has no " + kind};
    }
    if (found.size() > 1) {
        throw Error{std::string("ambiguous-") + kind,
                    std::string("document ") + doc.getName() + " has " +
                        std::to_string(found.size()) + " " + kind + "s, name one"};
    }
    return *found.front();
}

/// The sketch a request means when it did not name one: the newest in the
/// document, or the newest inside `body` when the request names one. Modelling
/// runs sequentially - `sketch new`, then geometry, then `pad` - so the last
/// sketch created is the one being drawn on, and every response echoes which
/// sketch it used.
Sketcher::SketchObject& sketch_for(App::Document& doc,
                                   const std::string& name,
                                   const PartDesign::Body* body = nullptr)
{
    if (!name.empty()) {
        return object_for<Sketcher::SketchObject>(doc, name, "sketch");
    }

    Sketcher::SketchObject* newest = nullptr;
    for (Sketcher::SketchObject* sketch : doc.getObjectsOfType<Sketcher::SketchObject>()) {
        if (body != nullptr && !body->hasObject(sketch)) {
            continue;
        }
        if (newest == nullptr || sketch->getID() > newest->getID()) {
            newest = sketch;
        }
    }

    if (newest == nullptr) {
        throw Error{"unknown-sketch",
                    body == nullptr
                        ? std::string("document ") + doc.getName() + " has no sketch"
                        : std::string("body ") + body->getNameInDocument() + " has no sketch"};
    }
    return *newest;
}

/// The feature(s) a mirror or pattern means: each one named, or - when none
/// were - the body's own tip, the feature the previous call in this vocabulary
/// just left behind. A named feature that belongs to another body is refused
/// rather than adopted, the same discipline `resolve_profile` already applies
/// to a foreign sketch.
std::vector<PartDesign::Feature*> features_for(App::Document& doc,
                                               const std::vector<std::string>& names,
                                               PartDesign::Body& body)
{
    if (names.empty()) {
        auto* tip = dynamic_cast<PartDesign::Feature*>(body.Tip.getValue());
        if (tip == nullptr) {
            throw Error{"unknown-feature",
                        std::string("body ") + body.getNameInDocument() + " has no feature yet"};
        }
        return {tip};
    }

    std::vector<PartDesign::Feature*> found;
    found.reserve(names.size());
    for (const std::string& name : names) {
        App::DocumentObject* object = doc.getObject(name.c_str());
        auto* feature = dynamic_cast<PartDesign::Feature*>(object);
        if (feature == nullptr) {
            throw Error{"unknown-feature", "no feature named " + name + " in " + doc.getName()};
        }
        if (PartDesign::Body::findBodyOf(feature) != &body) {
            throw Error{"foreign-feature", std::string("feature ") + name +
                                               " does not belong to " + body.getNameInDocument()};
        }
        found.push_back(feature);
    }
    return found;
}

/// Each `--tool` name resolved to the body it names. Unlike `features_for`,
/// there is no tip to fall back on - a boolean's tools are never implicit -
/// and a name that resolves to `base` itself is refused rather than folded
/// into a no-op.
std::vector<PartDesign::Body*> bodies_for(App::Document& doc,
                                          const std::vector<std::string>& names,
                                          const PartDesign::Body& base)
{
    std::vector<PartDesign::Body*> found;
    found.reserve(names.size());
    for (const std::string& name : names) {
        PartDesign::Body& body = object_for<PartDesign::Body>(doc, name, "body");
        if (&body == &base) {
            throw Error{"self-boolean", std::string("body ") + name + " cannot be its own tool"};
        }
        found.push_back(&body);
    }
    return found;
}

const char* point_pos_name(PointPos pos)
{
    switch (pos) {
        case PointPos::start: return "start";
        case PointPos::end: return "end";
        case PointPos::mid: return "mid";
        case PointPos::none:
        default: return "none";
    }
}

json::Value point(const Base::Vector3d& value)
{
    json::Value out = json::Value::object();
    out.set("x", json::Value::number(measure(value.x)));
    out.set("y", json::Value::number(measure(value.y)));
    return out;
}

json::Value vector3(const Base::Vector3d& value)
{
    json::Value out = json::Value::object();
    out.set("x", json::Value::number(measure(value.x)));
    out.set("y", json::Value::number(measure(value.y)));
    out.set("z", json::Value::number(measure(value.z)));
    return out;
}

json::Value vector3(const Vec3& value)
{
    json::Value out = json::Value::object();
    out.set("x", json::Value::number(measure(value.x)));
    out.set("y", json::Value::number(measure(value.y)));
    out.set("z", json::Value::number(measure(value.z)));
    return out;
}

/// Where the sketch's own u and v end up in global axes. Without this the
/// caller has to guess which way a sketch on XZ grows, and only finds out from
/// a wrong solid three commands later.
json::Value basis_of(const Sketcher::SketchObject& sketch)
{
    const Base::Placement placement = sketch.Placement.getValue();
    const Base::Rotation rotation = placement.getRotation();

    json::Value out = json::Value::object();
    out.set("origin", vector3(placement.getPosition()));
    out.set("x", vector3(rotation.multVec(Base::Vector3d(1.0, 0.0, 0.0))));
    out.set("y", vector3(rotation.multVec(Base::Vector3d(0.0, 1.0, 0.0))));
    out.set("normal", vector3(rotation.multVec(Base::Vector3d(0.0, 0.0, 1.0))));
    return out;
}

json::Value geometry_of(const Sketcher::SketchObject& sketch)
{
    json::Value out = json::Value::array();
    int index = 0;
    for (const Part::Geometry* geo : sketch.getInternalGeometry()) {
        const int at = index++;
        json::Value entry = json::Value::object();
        entry.set("index", json::Value::integer(at));
        entry.set("type", json::Value::string(geo->getTypeId().getName()));
        // Construction geometry carries the placement rather than the profile,
        // so a reader counting edges has to be able to skip it.
        entry.set("construction",
                  json::Value::boolean(sketch.getGeometryFacade(at)->getConstruction()));
        if (const auto* line = dynamic_cast<const Part::GeomLineSegment*>(geo)) {
            entry.set("start", point(line->getStartPoint()));
            entry.set("end", point(line->getEndPoint()));
            entry.set("length", json::Value::number(
                                    measure((line->getEndPoint() - line->getStartPoint())
                                                .Length())));
        }
        else if (const auto* circle = dynamic_cast<const Part::GeomCircle*>(geo)) {
            entry.set("centre", point(circle->getLocation()));
            entry.set("radius", json::Value::number(measure(circle->getRadius())));
        }
        else if (const auto* spot = dynamic_cast<const Part::GeomPoint*>(geo)) {
            entry.set("at", point(spot->getPoint()));
        }
        out.push(std::move(entry));
    }
    return out;
}

json::Value constraints_of(const Sketcher::SketchObject& sketch)
{
    json::Value out = json::Value::array();
    int index = 0;
    for (const Sketcher::Constraint* constraint : sketch.Constraints.getValues()) {
        json::Value entry = json::Value::object();
        entry.set("index", json::Value::integer(index++));
        entry.set("type", json::Value::string(constraint->typeToString()));
        entry.set("first", json::Value::integer(constraint->First));
        entry.set("first_pos", json::Value::string(point_pos_name(constraint->FirstPos)));
        if (constraint->Second != kGeoUndefined) {
            entry.set("second", json::Value::integer(constraint->Second));
            entry.set("second_pos", json::Value::string(point_pos_name(constraint->SecondPos)));
        }
        if (constraint->Third != kGeoUndefined) {
            entry.set("third", json::Value::integer(constraint->Third));
            entry.set("third_pos", json::Value::string(point_pos_name(constraint->ThirdPos)));
        }
        if (constraint->isDimensional()) {
            entry.set("value", json::Value::number(measure(constraint->getValue())));
        }
        entry.set("driving", json::Value::boolean(constraint->isDriving));
        if (!constraint->Name.empty()) {
            entry.set("name", json::Value::string(constraint->Name));
            const params::Binding binding =
                params::binding_of(sketch, "Constraints." + constraint->Name);
            if (!binding.expression.empty()) {
                entry.set("parameter", binding.parameter.empty()
                                           ? json::Value()
                                           : json::Value::string(binding.parameter));
                entry.set("expression", json::Value::string(binding.expression));
            }
        }
        out.push(std::move(entry));
    }
    return out;
}

/// Solving without updating geometry is what makes the degrees of freedom
/// readable from a freshly opened document without mutating it.
json::Value sketch_detail(Sketcher::SketchObject& sketch)
{
    sketch.solve(false);

    json::Value out = json::Value::object();
    out.set("plane", basis_of(sketch));
    out.set("geometry", geometry_of(sketch));
    out.set("constraints", constraints_of(sketch));
    out.set("dof", json::Value::integer(sketch.getLastDoF()));
    out.set("fully_constrained", json::Value::boolean(sketch.getLastDoF() == 0));
    out.set("redundant", json::Value::boolean(sketch.getLastHasRedundancies()));
    return out;
}

struct Box
{
    bool valid = false;
    double min[3] = {0.0, 0.0, 0.0};
    double max[3] = {0.0, 0.0, 0.0};

    void absorb(const Box& other)
    {
        if (!other.valid) {
            return;
        }
        if (!valid) {
            *this = other;
            return;
        }
        for (int axis = 0; axis < 3; ++axis) {
            min[axis] = std::min(min[axis], other.min[axis]);
            max[axis] = std::max(max[axis], other.max[axis]);
        }
    }
};

/// The exact box, not the fast one: `BRepBndLib::Add` inflates around curved
/// faces, and a bounding box nobody can trust to the micron is not a
/// measurement, it is a hint.
Box box_of(const TopoDS_Shape& shape)
{
    Box out;
    if (shape.IsNull()) {
        return out;
    }

    Bnd_Box bounds;
    BRepBndLib::AddOptimal(shape, bounds, Standard_False, Standard_False);
    if (bounds.IsVoid()) {
        return out;
    }
    bounds.SetGap(0.0);
    bounds.Get(out.min[0], out.min[1], out.min[2], out.max[0], out.max[1], out.max[2]);
    out.valid = true;
    return out;
}

json::Value box_json(const Box& box)
{
    static const char* axes[3] = {"x", "y", "z"};

    json::Value min = json::Value::object();
    json::Value max = json::Value::object();
    json::Value size = json::Value::object();
    json::Value centre = json::Value::object();
    for (int axis = 0; axis < 3; ++axis) {
        min.set(axes[axis], json::Value::number(measure(box.min[axis])));
        max.set(axes[axis], json::Value::number(measure(box.max[axis])));
        size.set(axes[axis], json::Value::number(measure(box.max[axis] - box.min[axis])));
        centre.set(axes[axis],
                   json::Value::number(measure(0.5 * (box.min[axis] + box.max[axis]))));
    }

    json::Value out = json::Value::object();
    out.set("min", std::move(min));
    out.set("max", std::move(max));
    out.set("size", std::move(size));
    out.set("centre", std::move(centre));
    return out;
}

bool has_solid(const TopoDS_Shape& shape)
{
    TopExp_Explorer solids(shape, TopAbs_SOLID);
    return solids.More() == Standard_True;
}

int solid_count(const TopoDS_Shape& shape)
{
    int count = 0;
    for (TopExp_Explorer solids(shape, TopAbs_SOLID); solids.More(); solids.Next()) {
        ++count;
    }
    return count;
}

double volume_of(const TopoDS_Shape& shape)
{
    if (shape.IsNull() || !has_solid(shape)) {
        return 0.0;
    }
    GProp_GProps volume;
    BRepGProp::VolumeProperties(shape, volume);
    return volume.Mass();
}

/// How much a body's running bounding box grew along each axis between two
/// points in its tree - the number a modeller can act on, where the running
/// box itself only ever grows monotonically as a side effect of PartDesign
/// keeping bodies cumulative.
json::Value bbox_delta_json(const Box& before, const Box& after)
{
    static const char* axes[3] = {"x", "y", "z"};
    json::Value out = json::Value::object();
    for (int axis = 0; axis < 3; ++axis) {
        const double before_size = before.valid ? before.max[axis] - before.min[axis] : 0.0;
        const double after_size = after.valid ? after.max[axis] - after.min[axis] : 0.0;
        out.set(axes[axis], json::Value::number(measure(after_size - before_size)));
    }
    return out;
}

/// What a caller has to be able to read to know they built a hammer and not an
/// axe: where the shape is, how big it is, how much of it there is.
json::Value shape_of(const Part::Feature& feature)
{
    const TopoDS_Shape& shape = feature.Shape.getValue();
    if (shape.IsNull()) {
        return json::Value();
    }

    const Box box = box_of(shape);
    if (!box.valid) {
        return json::Value();
    }

    json::Value out = box_json(box);

    GProp_GProps area;
    BRepGProp::SurfaceProperties(shape, area);
    out.set("area", json::Value::number(measure(area.Mass())));

    if (has_solid(shape)) {
        GProp_GProps volume;
        BRepGProp::VolumeProperties(shape, volume);
        out.set("volume", json::Value::number(measure(volume.Mass())));

        const gp_Pnt centre = volume.CentreOfMass();
        out.set("centre_of_mass",
                vector3(Base::Vector3d(centre.X(), centre.Y(), centre.Z())));
        out.set("solid", json::Value::boolean(true));
    }
    else {
        out.set("solid", json::Value::boolean(false));
    }
    return out;
}

/// Every body a `PartDesign::Boolean` has already folded into another body's
/// chain, keyed to the name of the boolean that did it. The tool body's own
/// object and feature history survive untouched - only its standing as a
/// shape of its own goes away, so a reader still finds it by name but never
/// counts it twice.
std::map<App::DocumentObject*, std::string> consumed_bodies(App::Document& doc)
{
    std::map<App::DocumentObject*, std::string> out;
    for (PartDesign::Boolean* boolean : doc.getObjectsOfType<PartDesign::Boolean>()) {
        for (App::DocumentObject* tool : boolean->Group.getValues()) {
            if (tool != nullptr) {
                out.emplace(tool, boolean->getNameInDocument());
            }
        }
    }
    return out;
}

/// A body resolved by name, or - when the caller did not name one - by being
/// the only body not already folded into another one by a Boolean. A tool
/// body a union has absorbed carries no shape of its own, so counting it
/// towards "how many bodies are there" would force every later call in the
/// document to name one just to work around a body nobody would ever mean.
PartDesign::Body& body_for(App::Document& doc, const std::string& name)
{
    if (!name.empty()) {
        return object_for<PartDesign::Body>(doc, name, "body");
    }

    std::vector<PartDesign::Body*> bodies = doc.getObjectsOfType<PartDesign::Body>();
    const std::map<App::DocumentObject*, std::string> consumed = consumed_bodies(doc);
    bodies.erase(std::remove_if(bodies.begin(), bodies.end(),
                                [&](PartDesign::Body* body) { return consumed.count(body) != 0; }),
                bodies.end());

    if (bodies.empty()) {
        throw Error{"unknown-body", std::string("document ") + doc.getName() + " has no body"};
    }
    if (bodies.size() > 1) {
        throw Error{"ambiguous-body",
                    std::string("document ") + doc.getName() + " has " +
                        std::to_string(bodies.size()) + " bodys, name one"};
    }
    return *bodies.front();
}

/// The shapes that make up the model as a reader sees it: every body not
/// folded into another one by a boolean, plus any loose solid feature that no
/// body owns. A pad inside a body is left out because the body already
/// carries its result.
std::vector<Part::Feature*> top_level_solids(App::Document& doc)
{
    std::vector<Part::Feature*> out;
    const std::map<App::DocumentObject*, std::string> consumed = consumed_bodies(doc);

    for (PartDesign::Body* body : doc.getObjectsOfType<PartDesign::Body>()) {
        if (consumed.count(body) != 0) {
            continue;
        }
        if (!body->Shape.getValue().IsNull()) {
            out.push_back(body);
        }
    }
    for (Part::Feature* feature : doc.getObjectsOfType<Part::Feature>()) {
        if (feature->isDerivedFrom(Part::Part2DObject::getClassTypeId()) ||
            feature->isDerivedFrom(PartDesign::Body::getClassTypeId())) {
            continue;
        }
        if (PartDesign::Body::findBodyOf(feature) != nullptr) {
            continue;
        }
        if (feature->Shape.getValue().IsNull()) {
            continue;
        }
        out.push_back(feature);
    }
    return out;
}

/// The shape a preview means: the body if the document has exactly one,
/// otherwise the single solid feature. Sketches are excluded even though they
/// are `Part::Feature`s, because a mesh of a sketch is empty.
Part::Feature& preview_target(App::Document& doc, const std::string& name)
{
    if (!name.empty()) {
        App::DocumentObject* object = doc.getObject(name.c_str());
        auto* feature = dynamic_cast<Part::Feature*>(object);
        if (feature == nullptr) {
            throw Error{"unknown-shape",
                        "no shape named " + name + " in " + doc.getName()};
        }
        return *feature;
    }

    std::vector<PartDesign::Body*> bodies = doc.getObjectsOfType<PartDesign::Body>();
    const std::map<App::DocumentObject*, std::string> consumed = consumed_bodies(doc);
    bodies.erase(std::remove_if(bodies.begin(), bodies.end(),
                                [&](PartDesign::Body* body) { return consumed.count(body) != 0; }),
                bodies.end());
    if (bodies.size() == 1) {
        return *bodies.front();
    }

    std::vector<Part::Feature*> solids;
    for (Part::Feature* feature : doc.getObjectsOfType<Part::Feature>()) {
        if (!feature->isDerivedFrom(Part::Part2DObject::getClassTypeId())) {
            solids.push_back(feature);
        }
    }
    if (solids.empty()) {
        throw Error{"unknown-shape", std::string("document ") + doc.getName() + " has no shape"};
    }
    if (solids.size() > 1) {
        throw Error{"ambiguous-shape",
                    std::string("document ") + doc.getName() + " has " +
                        std::to_string(solids.size()) + " shapes, name one"};
    }
    return *solids.front();
}

/// The errored objects are collected whether or not FreeCAD said the recompute
/// failed. It reports on what it touched, and a feature left in error by an
/// earlier edit is not touched again - a document that still answers every
/// query while one of its features is red is precisely the failure this exists
/// to make visible, so the flag is derived from the objects rather than
/// trusted.
json::Value recompute_document(App::Document& doc)
{
    bool failed = false;
    const int touched = doc.recompute({}, false, &failed);

    json::Value errors = json::Value::array();
    for (const App::DocumentObject* object : doc.getObjects()) {
        if (!object->isError()) {
            continue;
        }
        json::Value entry = json::Value::object();
        entry.set("object", json::Value::string(object->getNameInDocument()));
        entry.set("label", json::Value::string(object->Label.getValue()));
        entry.set("status", json::Value::string(object->getStatusString()));
        errors.push(std::move(entry));
    }

    const bool broken = failed || !errors.as_array()->empty();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("recomputed", json::Value::integer(touched));
    out.set("failed", json::Value::boolean(broken));
    out.set("errors", std::move(errors));
    return out;
}

/// A green recompute alone does not catch the poisoning defect: a fuse whose
/// only contact is a single point or an exact tangency is ACCEPTED by
/// FreeCAD, adds the right volume, and leaves the base and tool as two
/// separate solids in one compound rather than the single merged solid a
/// union promises - later operations expecting one solid return `Null
/// shape`. `solid_count` cannot tell that apart from an ordinary disjoint
/// gap on its own (both read as two valid, unconnected solids to
/// `BRepCheck_Analyzer`), so this is only meaningful for a union, whose
/// entire contract is producing one result - not for pad/pocket/revolve/loft
/// stacking onto a body's own chain, which a feature removal can legitimately
/// leave with a gap (see the `Stack` slice test). Only checked when the
/// previous tip was itself a clean single solid; a brand new body, or one
/// already broken by something else, has nothing valid to compare against,
/// so there is nothing this check could add.
/// `valid_additive_result` catches a union that leaves two solids where it
/// promised one, but a pad, revolve or loft that touches a body's own,
/// already-present material at a single point or edge instead of a face
/// still passes both of its signals: OCCT merges the touching pieces into
/// `solid_count == 1`, and `BRepCheck_Analyzer` calls the result valid - it
/// checks self-intersection and orientation, not whether every edge borders
/// the two faces a solid boundary requires. This checks that directly: build
/// edge->face and vertex->face incidence from the shape, and require every
/// edge to border exactly two faces. A pinch edge borders four. A pinch
/// vertex - the case a revolve's own apex singularity produces when it lands
/// on existing material - still only shows two faces on the edge check (its
/// own degenerate meridian edge is one of them), so the edges touching that
/// vertex are clustered by which faces they connect: material that meets
/// cleanly clusters into one group, two solids meeting at a point cluster
/// into two groups sharing no edge through it. Independent of `solid_count`,
/// so a body chain's deliberate gap (see the `Stack` slice test, where the
/// two solids never touch at all) leaves every edge and vertex cleanly
/// 2-manifold and passes.
bool is_manifold(const TopoDS_Shape& shape)
{
    TopTools_IndexedDataMapOfShapeListOfShape edge_faces;
    TopExp::MapShapesAndAncestors(shape, TopAbs_EDGE, TopAbs_FACE, edge_faces);
    for (int i = 1; i <= edge_faces.Extent(); ++i) {
        if (edge_faces.FindFromIndex(i).Extent() > 2) {
            return false;
        }
    }

    TopTools_IndexedDataMapOfShapeListOfShape vertex_faces;
    TopExp::MapShapesAndAncestors(shape, TopAbs_VERTEX, TopAbs_FACE, vertex_faces);
    TopTools_IndexedDataMapOfShapeListOfShape vertex_edges;
    TopExp::MapShapesAndAncestors(shape, TopAbs_VERTEX, TopAbs_EDGE, vertex_edges);

    for (int i = 1; i <= vertex_faces.Extent(); ++i) {
        const TopTools_ListOfShape& faces = vertex_faces.FindFromIndex(i);
        if (faces.Extent() < 2) {
            continue;
        }

        TopTools_IndexedMapOfShape face_index;
        for (TopTools_ListIteratorOfListOfShape it(faces); it.More(); it.Next()) {
            face_index.Add(it.Value());
        }

        std::vector<int> parent(static_cast<std::size_t>(face_index.Extent()) + 1);
        std::iota(parent.begin(), parent.end(), 0);
        std::function<int(int)> find = [&](int node) {
            while (parent[node] != node) {
                node = parent[node] = parent[parent[node]];
            }
            return node;
        };

        const TopTools_ListOfShape& edges = vertex_edges.FindFromKey(vertex_faces.FindKey(i));
        for (TopTools_ListIteratorOfListOfShape eit(edges); eit.More(); eit.Next()) {
            if (!edge_faces.Contains(eit.Value())) {
                continue;
            }
            int first = 0;
            const TopTools_ListOfShape& incident = edge_faces.FindFromKey(eit.Value());
            for (TopTools_ListIteratorOfListOfShape fit(incident); fit.More(); fit.Next()) {
                if (!face_index.Contains(fit.Value())) {
                    continue;
                }
                const int index = face_index.FindIndex(fit.Value());
                if (first == 0) {
                    first = index;
                }
                else {
                    const int a = find(first);
                    const int b = find(index);
                    if (a != b) {
                        parent[a] = b;
                    }
                }
            }
        }

        std::set<int> clusters;
        for (int f = 1; f <= face_index.Extent(); ++f) {
            clusters.insert(find(f));
        }
        if (clusters.size() > 1) {
            return false;
        }
    }
    return true;
}

bool valid_additive_result(PartDesign::Body& body, App::DocumentObject* previous_tip)
{
    auto* previous = dynamic_cast<Part::Feature*>(previous_tip);
    if (previous == nullptr || solid_count(previous->Shape.getValue()) != 1) {
        return true;
    }

    const TopoDS_Shape& shape = body.Shape.getValue();
    BRepCheck_Analyzer analyzer(shape);
    return analyzer.IsValid() == Standard_True && solid_count(shape) == 1;
}

/// What every creating verb needs after `body.addObject(feature)`: recompute,
/// and on failure take the feature back out rather than let it survive red in
/// the tree. Mirrors apply_removal's tip-restore + removeObject order, since a
/// half-built feature leaving the tip pointed at it is the same problem a
/// removed one is. `additive` opts a union into `valid_additive_result`'s
/// solid-count check above, which only makes sense for a union's
/// exactly-one-result contract. `check_manifold` opts in the pinch check
/// instead - meaningful for any creating verb that can touch existing
/// material (pad, revolve, loft, union), not just union.
json::Value recompute_or_rollback(App::Document& doc, PartDesign::Body& body,
                                   App::DocumentObject* previous_tip,
                                   PartDesign::Feature& created,
                                   bool additive = false,
                                   bool check_manifold = false)
{
    json::Value recomputed = recompute_document(doc);
    const bool clean = recomputed.find("failed")->as_bool() != true;
    const bool bad_union = clean && additive && !valid_additive_result(body, previous_tip);
    const bool pinched = clean && check_manifold && !is_manifold(body.Shape.getValue());
    const bool degenerate = bad_union || pinched;
    if (clean && !degenerate) {
        return recomputed;
    }

    std::string status;
    if (degenerate) {
        status = "the new material only touches the existing solid at a degenerate "
                 "point or tangent surface, leaving a non-manifold result";
    }
    else {
        // The break is not always in `created`: splicing a dressup into the
        // middle of a chain can leave a downstream feature - one `created`
        // itself never touches - unable to rebuild on top of it. Prefer
        // `created`'s own error when it has one; otherwise name whichever
        // feature actually broke rather than reporting `created`'s own
        // (unhelpfully "Valid") status.
        const std::vector<json::Value>& errors = *recomputed.find("errors")->as_array();
        for (const json::Value& entry : errors) {
            if (*entry.find("object")->as_string() == created.getNameInDocument()) {
                status = *entry.find("status")->as_string();
                break;
            }
        }
        if (status.empty() && !errors.empty()) {
            status = *errors.front().find("object")->as_string() + ": " +
                      *errors.front().find("status")->as_string();
        }
        if (status.empty()) {
            status = created.getStatusString();
        }
    }

    body.Tip.setValue(previous_tip);
    body.removeObject(&created);
    doc.removeObject(created.getNameInDocument());
    (void)recompute_document(doc);

    throw Error{degenerate ? "degenerate-contact" : "recompute-failed", status};
}

json::Value bounds_of(const Part::Feature& feature)
{
    const Box box = box_of(feature.Shape.getValue());

    json::Value out = json::Value::object();
    out.set("x", json::Value::number(measure(box.max[0] - box.min[0])));
    out.set("y", json::Value::number(measure(box.max[1] - box.min[1])));
    out.set("z", json::Value::number(measure(box.max[2] - box.min[2])));
    return out;
}

json::Value document_summary(const App::Document& doc)
{
    json::Value out = json::Value::object();
    // `document`, not `name`: every other verb echoes the document it acted on
    // under that key, and `inspect` also carries an object list whose entries
    // each have a `name` of their own.
    out.set("document", json::Value::string(doc.getName()));
    out.set("label", json::Value::string(doc.Label.getValue()));
    const char* file = doc.getFileName();
    out.set("file", (file != nullptr && *file != '\0') ? json::Value::string(file) : json::Value());
    out.set("objects", json::Value::integer(static_cast<long long>(doc.getObjects().size())));
    return out;
}

/// Sketcher stores a name on the constraint itself and writes it into the
/// saved file, so a dimension named here is still named after a round trip.
void name_constraint(Sketcher::SketchObject& sketch, int index, const std::string& name)
{
    if (name.empty()) {
        return;
    }

    const std::vector<Sketcher::Constraint*>& all = sketch.Constraints.getValues();
    if (index < 0 || index >= static_cast<int>(all.size())) {
        throw Error{"internal", "named a constraint that does not exist"};
    }

    std::vector<Sketcher::Constraint*> copy;
    copy.reserve(all.size());
    for (const Sketcher::Constraint* constraint : all) {
        copy.push_back(constraint->clone());
    }
    copy[static_cast<std::size_t>(index)]->Name = name;
    sketch.Constraints.setValues(std::move(copy));
}

/// The canonical slot name when nothing holds it yet, else name_2, name_3, ...
/// A sketch holds several primitives, and `constraint_named` answers the first
/// match, so a second "radius" would be a dimension no verb could ever reach.
std::string unique_name(const Sketcher::SketchObject& sketch, const std::string& base)
{
    auto taken = [&sketch](const std::string& name) {
        for (const Sketcher::Constraint* constraint : sketch.Constraints.getValues()) {
            if (constraint->Name == name) {
                return true;
            }
        }
        return false;
    };

    if (!taken(base)) {
        return base;
    }
    for (int k = 2;; ++k) {
        const std::string candidate = base + "_" + std::to_string(k);
        if (!taken(candidate)) {
            return candidate;
        }
    }
}

/// Canonical name to the name this call actually got: `{"x": "x_3", ...}`. The
/// second primitive's dimensions carry suffixed names, and a caller that wants
/// to drive one has to be told which, not left to guess the suffix.
json::Value slot_names(std::initializer_list<std::pair<const char*, std::string>> names)
{
    json::Value out = json::Value::object();
    for (const auto& [canonical, actual] : names) {
        out.set(canonical, json::Value::string(actual));
    }
    return out;
}

/// Pins one sketch point to the origin with a named signed distance on each
/// axis. Always a pair, never the tidier coincidence a point already at the
/// origin would allow: an unnamed constraint cannot be bound to a parameter
/// afterwards, and a placement that can only be decided while drawing is the
/// same write-once defect one level up from the values it holds.
void pin_to_origin(Sketcher::SketchObject& sketch, int geo, PointPos pos, double x, double y,
                   const std::string& name_x, const std::string& name_y)
{
    auto constrain = [&sketch](Sketcher::ConstraintType type, int second, PointPos second_pos,
                               double value) {
        Sketcher::Constraint constraint;
        constraint.Type = type;
        constraint.First = kRootPoint;
        constraint.FirstPos = PointPos::start;
        constraint.Second = second;
        constraint.SecondPos = second_pos;
        constraint.setValue(value);
        return sketch.addConstraint(&constraint);
    };

    name_constraint(sketch, constrain(Sketcher::DistanceX, geo, pos, x), name_x);
    name_constraint(sketch, constrain(Sketcher::DistanceY, geo, pos, y), name_y);
}

int constraint_named(const Sketcher::SketchObject& sketch, const std::string& name)
{
    int index = 0;
    for (const Sketcher::Constraint* constraint : sketch.Constraints.getValues()) {
        const int at = index++;
        if (constraint->Name == name && constraint->isDimensional()) {
            return at;
        }
    }
    return -1;
}

/// The sketch slots that are not constraints. Everything else is a named
/// dimension addressed by its own name, which is also how a document written
/// before the registry stays reachable: an old `--name-width bar_w` is simply a
/// slot called bar_w.
/// `param list` addresses a slot the way FreeCAD stores it, and this is the
/// way back. Whatever the readback printed can be pasted straight into `slot
/// set`, so naming a dependency and undoing it are the same vocabulary.
std::string sketch_slot_alias(const std::string& slot)
{
    if (slot == "AttachmentOffset.Base.x") {
        return "offset_x";
    }
    if (slot == "AttachmentOffset.Base.y") {
        return "offset_y";
    }
    if (slot == "AttachmentOffset.Base.z") {
        return "offset_z";
    }
    if (slot == "AttachmentOffset.Rotation.Angle") {
        return "rotate";
    }
    if (slot.rfind("Constraints.", 0) == 0) {
        return slot.substr(std::string("Constraints.").size());
    }
    return slot;
}

std::string sketch_slot_path(const std::string& slot)
{
    if (slot == "offset_x") {
        return "AttachmentOffset.Base.x";
    }
    if (slot == "offset_y") {
        return "AttachmentOffset.Base.y";
    }
    if (slot == "offset_z") {
        return "AttachmentOffset.Base.z";
    }
    if (slot == "rotate") {
        return "AttachmentOffset.Rotation.Angle";
    }
    return "Constraints." + slot;
}

/// Signed degrees about the sketch normal. `Rotation::getAngle` is unsigned and
/// pushes the sign into the axis, so the turned x axis is read directly rather
/// than trusting which way FreeCAD chose to normalise.
double rotation_degrees(const Base::Placement& placement)
{
    const Base::Vector3d turned = placement.getRotation().multVec(Base::Vector3d(1.0, 0.0, 0.0));
    return std::atan2(turned.y, turned.x) * 180.0 / M_PI;
}

double sketch_slot_value(Sketcher::SketchObject& sketch, const std::string& slot)
{
    const Base::Placement offset = sketch.AttachmentOffset.getValue();
    if (slot == "offset_x") {
        return offset.getPosition().x;
    }
    if (slot == "offset_y") {
        return offset.getPosition().y;
    }
    if (slot == "offset_z") {
        return offset.getPosition().z;
    }
    if (slot == "rotate") {
        return rotation_degrees(offset);
    }

    const int at = constraint_named(sketch, slot);
    if (at < 0) {
        throw Error{"unknown-slot", std::string("sketch ") + sketch.getNameInDocument() +
                                        " has no dimension named " + slot};
    }
    return sketch.Constraints.getValues()[static_cast<std::size_t>(at)]->getValue();
}

void set_sketch_slot(Sketcher::SketchObject& sketch, const std::string& slot, double value)
{
    if (slot.rfind("offset_", 0) == 0 || slot == "rotate") {
        Base::Placement offset = sketch.AttachmentOffset.getValue();
        Base::Vector3d position = offset.getPosition();
        if (slot == "offset_x") {
            position.x = value;
        }
        else if (slot == "offset_y") {
            position.y = value;
        }
        else if (slot == "offset_z") {
            position.z = value;
        }
        offset.setPosition(position);
        if (slot == "rotate") {
            offset.setRotation(
                Base::Rotation(Base::Vector3d(0.0, 0.0, 1.0), value * M_PI / 180.0));
        }
        sketch.AttachmentOffset.setValue(offset);
        return;
    }

    const int at = constraint_named(sketch, slot);
    if (at < 0) {
        throw Error{"unknown-slot", std::string("sketch ") + sketch.getNameInDocument() +
                                        " has no dimension named " + slot};
    }
    if (sketch.setDatum(at, value) != 0) {
        throw Error{"unsolvable", "the sketch does not solve with " + slot + " set to " +
                                      std::to_string(value)};
    }
}

/// What FreeCAD marks on an object it could not build. Reported everywhere a
/// model is read back: a recompute that half failed leaves a document that
/// still answers every query, and an agent driving one parameter into six
/// features has no other way to find out.
json::Value error_of(const App::DocumentObject& object)
{
    return object.isError() ? json::Value::string(object.getStatusString()) : json::Value();
}

/// Both sketch primitives answer the same way, so a caller reads dof and
/// constraint count off whichever one it drew.
void append_sketch_detail(json::Value& out, Sketcher::SketchObject& sketch)
{
    const json::Value detail = sketch_detail(sketch);
    for (const char* field :
         {"plane", "geometry", "constraints", "dof", "fully_constrained", "redundant"}) {
        const json::Value* value = detail.find(field);
        if (value != nullptr) {
            out.set(field, *value);
        }
    }
}

/// Every named dimension of a sketch with what drives it. The feature tree is
/// only a complete description if the numbers inside it say whether they will
/// move, so this is not a summary of the constraint list but the part of it a
/// caller can act on.
json::Value dimensions_of(Sketcher::SketchObject& sketch)
{
    json::Value out = json::Value::array();
    for (const Sketcher::Constraint* constraint : sketch.Constraints.getValues()) {
        if (constraint->Name.empty() || !constraint->isDimensional()) {
            continue;
        }
        json::Value entry = params::slot_json(sketch, "Constraints." + constraint->Name,
                                              measure(constraint->getValue()));
        entry.set("slot", json::Value::string(constraint->Name));
        entry.set("type", json::Value::string(constraint->typeToString()));
        out.push(std::move(entry));
    }
    return out;
}

/// The profile as the feature consumed it: which plane, how far off it, and
/// which of those numbers a parameter holds.
json::Value profile_of(Sketcher::SketchObject& sketch)
{
    sketch.solve(false);
    const Base::Placement offset = sketch.AttachmentOffset.getValue();

    json::Value moved = json::Value::object();
    moved.set("x", params::slot_json(sketch, "AttachmentOffset.Base.x",
                                     measure(offset.getPosition().x)));
    moved.set("y", params::slot_json(sketch, "AttachmentOffset.Base.y",
                                     measure(offset.getPosition().y)));
    moved.set("z", params::slot_json(sketch, "AttachmentOffset.Base.z",
                                     measure(offset.getPosition().z)));
    moved.set("rotate", params::slot_json(sketch, "AttachmentOffset.Rotation.Angle",
                                          measure(rotation_degrees(offset))));

    const App::DocumentObject* support = sketch.AttachmentSupport.getValue();

    json::Value out = json::Value::object();
    out.set("name", json::Value::string(sketch.getNameInDocument()));
    out.set("plane", support != nullptr ? json::Value::string(support->getNameInDocument())
                                        : json::Value());
    out.set("offset", std::move(moved));
    out.set("basis", basis_of(sketch));
    out.set("primitives", geometry_of(sketch));
    out.set("dimensions", dimensions_of(sketch));
    out.set("dof", json::Value::integer(sketch.getLastDoF()));
    out.set("fully_constrained", json::Value::boolean(sketch.getLastDoF() == 0));
    return out;
}

/// What revolve and groove share in an `inspect --features` entry: same
/// property names on both classes, so one function reads either.
template<class Feature>
void append_revolve_detail(json::Value& out, Feature& feature, const char* kind)
{
    out.set("kind", json::Value::string(kind));
    out.set("angle", params::slot_json(feature, "Angle", measure(feature.Angle.getValue())));
    out.set("midplane", json::Value::boolean(feature.Midplane.getValue()));
    out.set("reversed", json::Value::boolean(feature.Reversed.getValue()));
    if (auto* profile = dynamic_cast<Sketcher::SketchObject*>(feature.Profile.getValue())) {
        out.set("sketch", profile_of(*profile));
    }
}

/// The originals a `Transformed` feature replicates, by name - a mirror or
/// pattern does not consume a sketch, it replicates another feature's shape.
json::Value originals_of(const PartDesign::Transformed& transformed)
{
    json::Value out = json::Value::array();
    for (App::DocumentObject* original : transformed.Originals.getValues()) {
        if (original != nullptr) {
            out.push(json::Value::string(original->getNameInDocument()));
        }
    }
    return out;
}

json::Value feature_of(App::DocumentObject& object)
{
    json::Value out = json::Value::object();
    out.set("name", json::Value::string(object.getNameInDocument()));
    out.set("type", json::Value::string(object.getTypeId().getName()));
    out.set("label", json::Value::string(object.Label.getValue()));
    out.set("error", error_of(object));
    if (auto* solid = dynamic_cast<Part::Feature*>(&object)) {
        json::Value shape = shape_of(*solid);
        if (!shape.is_null()) {
            out.set("shape", std::move(shape));
        }
    }

    if (auto* revolution = dynamic_cast<PartDesign::Revolution*>(&object)) {
        append_revolve_detail(out, *revolution, "revolve");
        return out;
    }
    if (auto* groove = dynamic_cast<PartDesign::Groove*>(&object)) {
        append_revolve_detail(out, *groove, "groove");
        return out;
    }
    if (auto* mirrored = dynamic_cast<PartDesign::Mirrored*>(&object)) {
        out.set("kind", json::Value::string("mirror"));
        if (auto* plane = mirrored->MirrorPlane.getValue()) {
            out.set("plane", json::Value::string(plane->getNameInDocument()));
        }
        out.set("features", originals_of(*mirrored));
        return out;
    }
    if (auto* linear = dynamic_cast<PartDesign::LinearPattern*>(&object)) {
        out.set("kind", json::Value::string("pattern_linear"));
        if (auto* direction = linear->Direction.getValue()) {
            out.set("direction", json::Value::string(direction->getNameInDocument()));
        }
        out.set("spacing",
                params::slot_json(*linear, "Offset", measure(linear->Offset.getValue())));
        out.set("count", json::Value::integer(linear->Occurrences.getValue()));
        out.set("features", originals_of(*linear));
        return out;
    }
    if (auto* polar = dynamic_cast<PartDesign::PolarPattern*>(&object)) {
        out.set("kind", json::Value::string("pattern_polar"));
        if (auto* axis = polar->Axis.getValue()) {
            out.set("axis", json::Value::string(axis->getNameInDocument()));
        }
        out.set("angle", params::slot_json(*polar, "Angle", measure(polar->Angle.getValue())));
        out.set("count", json::Value::integer(polar->Occurrences.getValue()));
        out.set("features", originals_of(*polar));
        return out;
    }
    if (auto* fillet = dynamic_cast<PartDesign::Fillet*>(&object)) {
        out.set("kind", json::Value::string("fillet"));
        if (auto* base = fillet->Base.getValue()) {
            out.set("base", json::Value::string(base->getNameInDocument()));
        }
        out.set("radius",
                params::slot_json(*fillet, "Radius", measure(fillet->Radius.getValue())));
        out.set("edges", json::Value::integer(
                             static_cast<long long>(fillet->Base.getSubValues().size())));
        return out;
    }
    if (auto* chamfer = dynamic_cast<PartDesign::Chamfer*>(&object)) {
        out.set("kind", json::Value::string("chamfer"));
        if (auto* base = chamfer->Base.getValue()) {
            out.set("base", json::Value::string(base->getNameInDocument()));
        }
        out.set("size", params::slot_json(*chamfer, "Size", measure(chamfer->Size.getValue())));
        if (std::string(chamfer->ChamferType.getValueAsString()) == "Distance and Angle") {
            out.set("angle",
                    params::slot_json(*chamfer, "Angle", measure(chamfer->Angle.getValue())));
        }
        out.set("edges", json::Value::integer(
                             static_cast<long long>(chamfer->Base.getSubValues().size())));
        return out;
    }
    if (auto* boolean = dynamic_cast<PartDesign::Boolean*>(&object)) {
        out.set("kind", json::Value::string("boolean"));
        const std::string type = boolean->Type.getValueAsString();
        out.set("operation", json::Value::string(
                                 type == "Fuse" ? "union" : type == "Cut" ? "cut" : "intersect"));
        if (auto* base = boolean->BaseFeature.getValue()) {
            out.set("base", json::Value::string(base->getNameInDocument()));
        }
        json::Value tool = json::Value::array();
        for (App::DocumentObject* body : boolean->Group.getValues()) {
            if (body != nullptr) {
                tool.push(json::Value::string(body->getNameInDocument()));
            }
        }
        out.set("tool", std::move(tool));
        return out;
    }

    if (auto* loft = dynamic_cast<PartDesign::Loft*>(&object)) {
        out.set("kind", json::Value::string(loft->isDerivedFrom(
                                                 PartDesign::SubtractiveLoft::getClassTypeId())
                                                 ? "loft_pocket"
                                                 : "loft_new"));
        out.set("ruled", json::Value::boolean(loft->Ruled.getValue()));
        out.set("closed", json::Value::boolean(loft->Closed.getValue()));
        json::Value sketches = json::Value::array();
        if (auto* profile = dynamic_cast<Sketcher::SketchObject*>(loft->Profile.getValue())) {
            sketches.push(json::Value::string(profile->getNameInDocument()));
        }
        for (App::DocumentObject* section : loft->Sections.getValues()) {
            if (auto* sketch = dynamic_cast<Sketcher::SketchObject*>(section)) {
                sketches.push(json::Value::string(sketch->getNameInDocument()));
            }
        }
        out.set("sketches", std::move(sketches));
        return out;
    }

    auto* extrude = dynamic_cast<PartDesign::FeatureExtrude*>(&object);
    if (extrude == nullptr) {
        return out;
    }

    const bool cutting = extrude->isDerivedFrom(PartDesign::Pocket::getClassTypeId());
    const std::string depth = extrude->Type.getValueAsString();
    out.set("kind", json::Value::string(cutting ? "pocket" : "pad"));
    out.set("length",
            params::slot_json(*extrude, "Length", measure(extrude->Length.getValue())));
    out.set("through_all", json::Value::boolean(depth == "ThroughAll"));
    out.set("midplane",
            json::Value::boolean(std::string(extrude->SideType.getValueAsString()) ==
                                 "Symmetric"));
    out.set("reversed", json::Value::boolean(extrude->Reversed.getValue()));
    out.set("taper",
            params::slot_json(*extrude, "TaperAngle", measure(extrude->TaperAngle.getValue())));
    if (auto* profile = dynamic_cast<Sketcher::SketchObject*>(extrude->Profile.getValue())) {
        out.set("sketch", profile_of(*profile));
    }
    return out;
}

/// FreeCAD's own scaffolding: an origin and its three planes and three axes per
/// body, none of them ever addressed by name. Seven entries per body buried the
/// four a reader was looking for.
bool is_scaffolding(const App::DocumentObject& object)
{
    return object.isDerivedFrom(App::Origin::getClassTypeId()) ||
           object.isDerivedFrom(App::Plane::getClassTypeId()) ||
           object.isDerivedFrom(App::Line::getClassTypeId()) ||
           object.isDerivedFrom(App::Point::getClassTypeId());
}

PartDesign::FeatureExtrude& extrude_for(App::Document& doc,
                                        const std::string& name,
                                        const std::string& kind)
{
    if (kind == "pocket") {
        return object_for<PartDesign::Pocket>(doc, name, "pocket");
    }
    return object_for<PartDesign::Pad>(doc, name, "pad");
}

App::DocumentObject& revolve_for(App::Document& doc,
                                 const std::string& name,
                                 const std::string& kind)
{
    if (kind == "groove") {
        return object_for<PartDesign::Groove>(doc, name, "groove");
    }
    return object_for<PartDesign::Revolution>(doc, name, "revolve");
}

App::PropertyAngle& angle_property(App::DocumentObject& object)
{
    auto* angle = dynamic_cast<App::PropertyAngle*>(object.getPropertyByName("Angle"));
    if (angle == nullptr) {
        throw Error{"unknown-slot",
                    std::string(object.getNameInDocument()) + " has no Angle property"};
    }
    return *angle;
}

/// Every name, never the first one. A refusal that reports a sample turns one
/// repair into as many round trips as there are holders, and every other report
/// here names the whole set: `param set` lists every slot it drives, removal
/// lists every relink and every sketch it leaves behind.
std::string listed(const std::vector<std::string>& names)
{
    std::string out;
    for (const std::string& name : names) {
        if (!out.empty()) {
            out += ", ";
        }
        out += name;
    }
    return out;
}

json::Value strings_json(const std::vector<std::string>& names)
{
    json::Value out = json::Value::array();
    for (const std::string& name : names) {
        out.push(json::Value::string(name));
    }
    return out;
}

/// Everything one removal touches, worked out before anything is touched. The
/// dry run reports this and stops there; the real run applies these exact
/// fields and reports the same value, so the preview cannot describe an edit
/// different from the one that happens.
struct RemovalPlan
{
    App::DocumentObject* object = nullptr;
    PartDesign::Body* body = nullptr;
    /// What the removed feature was built on. The successor inherits it, and
    /// when the tip moves it moves to the same place: one link answers both.
    App::DocumentObject* base = nullptr;
    /// The feature whose BaseFeature points at the one going away. FreeCAD
    /// clears that link rather than repairing it, and a body whose chain has a
    /// hole in it recomputes to the material below the hole and reports
    /// Up-to-date, so this is the load-bearing half of the verb.
    PartDesign::Feature* successor = nullptr;
    bool moves_tip = false;
    /// The profile the feature consumed, which removal leaves in the document.
    Sketcher::SketchObject* widowed = nullptr;
    /// Parameters whose every slot is on the object going away. They survive
    /// the removal driving nothing, which is recoverable, unlike a parameter
    /// deleted on the model's behalf.
    std::vector<std::string> orphaned;
};

RemovalPlan plan_removal(App::Document& doc, const std::string& name)
{
    if (name.empty()) {
        throw Error{"missing-feature",
                    "name what to remove: every other verb may resolve an unnamed object, "
                    "this one may not guess at one"};
    }

    App::DocumentObject* object = doc.getObject(name.c_str());
    if (object == nullptr) {
        throw Error{"unknown-feature",
                    "no object named " + name + " in " + doc.getName() +
                        ": `document inspect --features` names them"};
    }
    if (is_scaffolding(*object)) {
        throw Error{"unremovable",
                    name + " is one of the body's origin planes, which FreeCAD owns"};
    }
    if (object->isDerivedFrom(App::VarSet::getClassTypeId())) {
        throw Error{"unremovable",
                    name + " is the parameter registry: `param remove <name>` takes one "
                           "parameter out of it"};
    }
    if (object->isDerivedFrom(PartDesign::Body::getClassTypeId())) {
        throw Error{"unremovable",
                    name + " is a body. What removing one means depends on whether another "
                           "body is built from it, so it waits for booleans rather than "
                           "guessing now"};
    }

    RemovalPlan plan;
    plan.object = object;
    plan.body = PartDesign::Body::findBodyOf(object);

    if (object->isDerivedFrom(Sketcher::SketchObject::getClassTypeId())) {
        // Removing it anyway leaves the holder with Profile: None, an Invalid
        // status and the shape it last built - so the body still looks right
        // while being unrebuildable. A verb that knowingly creates that and
        // reports it afterwards is worse than one that will not create it.
        std::vector<std::string> holders;
        for (App::DocumentObject* holder : doc.getObjects()) {
            auto* based = dynamic_cast<PartDesign::ProfileBased*>(holder);
            if (based != nullptr && based->Profile.getValue() == object) {
                holders.push_back(holder->getNameInDocument());
            }
        }
        if (!holders.empty()) {
            throw Error{"sketch-in-use",
                        name + " is the profile of " + listed(holders) + ": remove " +
                            (holders.size() == 1 ? "that feature" : "those features") +
                            " first, which leaves this sketch behind"};
        }
    }
    else if (auto* feature = dynamic_cast<PartDesign::Feature*>(object)) {
        plan.base = feature->BaseFeature.getValue();
        for (App::DocumentObject* member : doc.getObjects()) {
            auto* later = dynamic_cast<PartDesign::Feature*>(member);
            if (later != nullptr && later != feature &&
                later->BaseFeature.getValue() == feature) {
                plan.successor = later;
                break;
            }
        }
        if (auto* based = dynamic_cast<PartDesign::ProfileBased*>(feature)) {
            plan.widowed = dynamic_cast<Sketcher::SketchObject*>(based->Profile.getValue());
        }
        plan.moves_tip = plan.body != nullptr && plan.body->Tip.getValue() == feature;
    }
    else {
        throw Error{"unremovable",
                    name + " is a " + object->getTypeId().getName() +
                        ", which this verb does not take"};
    }

    const std::vector<std::string> followers = params::following(doc, *object);
    if (!followers.empty()) {
        throw Error{"parameter-follows",
                    name + " is read by the expression on " +
                        (followers.size() == 1 ? "parameter " : "parameters ") +
                        listed(followers) + ": point " +
                        (followers.size() == 1 ? "it" : "each of them") +
                        " somewhere else first, because nothing here can rewrite arithmetic "
                        "somebody wrote"};
    }

    for (const std::string& parameter : params::names(doc)) {
        const json::Value bound = params::drives(doc, parameter);
        const std::vector<json::Value>* slots = bound.as_array();
        if (slots == nullptr || slots->empty()) {
            continue;
        }
        const bool all_here = std::all_of(slots->begin(), slots->end(), [&](const auto& slot) {
            return *slot.find("object")->as_string() == name;
        });
        if (all_here) {
            plan.orphaned.push_back(parameter);
        }
    }

    return plan;
}

void apply_removal(App::Document& doc, const RemovalPlan& plan)
{
    if (plan.successor != nullptr) {
        plan.successor->BaseFeature.setValue(plan.base);
    }
    if (plan.moves_tip) {
        // Null when the body empties out, and that is the honest value: a body
        // with no tip has no shape at all, so `Shape` raises rather than
        // returning a stale or empty one.
        plan.body->Tip.setValue(plan.base);
    }
    // Only the bookkeeping is left to FreeCAD - erasing the entry from Group.
    // Its own relink and tip repair read the same two links the plan did, and
    // the plan has already written them, so they find nothing to do.
    if (plan.body != nullptr) {
        plan.body->removeObject(plan.object);
    }
    doc.removeObject(plan.object->getNameInDocument());
}

json::Value plan_json(const RemovalPlan& plan)
{
    json::Value relinked = json::Value::array();
    if (plan.successor != nullptr) {
        json::Value entry = json::Value::object();
        entry.set("object", json::Value::string(plan.successor->getNameInDocument()));
        entry.set("slot", json::Value::string("BaseFeature"));
        entry.set("to", plan.base == nullptr
                            ? json::Value()
                            : json::Value::string(plan.base->getNameInDocument()));
        relinked.push(std::move(entry));
    }

    json::Value left_behind = json::Value::array();
    if (plan.widowed != nullptr) {
        json::Value entry = json::Value::object();
        entry.set("object", json::Value::string(plan.widowed->getNameInDocument()));
        entry.set("slot", json::Value::string("Profile"));
        left_behind.push(std::move(entry));
    }

    json::Value orphaned = json::Value::array();
    for (const std::string& parameter : plan.orphaned) {
        orphaned.push(json::Value::string(parameter));
    }

    json::Value out = json::Value::object();
    out.set("removed", json::Value::string(plan.object->getNameInDocument()));
    out.set("type", json::Value::string(plan.object->getTypeId().getName()));
    out.set("label", json::Value::string(plan.object->Label.getValue()));
    out.set("body", plan.body == nullptr
                        ? json::Value()
                        : json::Value::string(plan.body->getNameInDocument()));
    out.set("relinked", std::move(relinked));
    out.set("tip", plan.moves_tip ? (plan.base == nullptr
                                         ? json::Value()
                                         : json::Value::string(plan.base->getNameInDocument()))
                                  : json::Value());
    out.set("tip_moves", json::Value::boolean(plan.moves_tip));
    out.set("left_behind", std::move(left_behind));
    out.set("orphaned", std::move(orphaned));
    return out;
}

}  // namespace

json::Value Session::status() const
{
    const auto& config = App::Application::Config();
    auto value_of = [&config](const char* key) {
        auto found = config.find(key);
        return found == config.end() ? std::string() : found->second;
    };

    json::Value freecad = json::Value::object();
    freecad.set("version", json::Value::string(value_of("BuildVersionMajor") + "." +
                                               value_of("BuildVersionMinor") + "." +
                                               value_of("BuildVersionPoint")));
    freecad.set("home", json::Value::string(value_of("AppHomePath")));

    json::Value documents = json::Value::array();
    for (const App::Document* doc : App::GetApplication().getDocuments()) {
        documents.push(document_summary(*doc));
    }

    const App::Document* active = App::GetApplication().getActiveDocument();

    json::Value previews = json::Value::array();
    for (const auto& [document, followed] : followed_) {
        json::Value entry = json::Value::object();
        entry.set("document", json::Value::string(document));
        entry.set("object", json::Value::string(followed.object));
        entry.set("path", json::Value::string(followed.path));
        previews.push(std::move(entry));
    }

    json::Value unsaved = json::Value::array();
    for (const std::string& name : unsaved_documents()) {
        unsaved.push(json::Value::string(name));
    }

    json::Value idle = json::Value::object();
    idle.set("timeout", json::Value::integer(idle_timeout_));
    idle.set("blocked", json::Value::boolean(!unsaved_documents().empty()));

    json::Value out = json::Value::object();
    out.set("mode", json::Value::string(gui::active() ? "gui" : "headless"));
    out.set("build", json::Value::string(build_id()));
    out.set("freecad", std::move(freecad));
    out.set("active", active != nullptr ? json::Value::string(active->getName()) : json::Value());
    out.set("documents", std::move(documents));
    out.set("previews", std::move(previews));
    out.set("unsaved", std::move(unsaved));
    out.set("idle", std::move(idle));
    return out;
}

std::vector<std::string> Session::unsaved_documents() const
{
    std::vector<std::string> out;
    for (const std::string& name : unsaved_) {
        if (App::GetApplication().getDocument(name.c_str()) != nullptr) {
            out.push_back(name);
        }
    }
    return out;
}

json::Value Session::new_document(const std::string& name)
{
    const std::string wanted = name.empty() ? std::string("Unnamed") : name;
    App::Document* doc = App::GetApplication().newDocument(wanted.c_str(), wanted.c_str());
    if (doc == nullptr) {
        throw Error{"document-failed", "FreeCAD refused to create document " + wanted};
    }
    App::GetApplication().setActiveDocument(doc);
    gui::reset_view();
    return document_summary(*doc);
}

json::Value Session::open_document(const std::string& path)
{
    if (path.empty()) {
        throw Error{"missing-path", "open needs a file path"};
    }
    App::Document* doc = App::GetApplication().openDocument(path.c_str());
    if (doc == nullptr) {
        throw Error{"open-failed", "FreeCAD could not open " + path};
    }
    App::GetApplication().setActiveDocument(doc);
    return document_summary(*doc);
}

json::Value Session::new_body(const std::string& document, const std::string& name)
{
    App::Document& doc = document_for(document);
    const std::string wanted = name.empty() ? std::string("Body") : name;

    auto* body = static_cast<PartDesign::Body*>(
        doc.addObject("PartDesign::Body", wanted.c_str()));
    if (body == nullptr) {
        throw Error{"body-failed", "FreeCAD refused to create a PartDesign::Body"};
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body->getNameInDocument()));
    out.set("label", json::Value::string(body->Label.getValue()));
    return out;
}

json::Value Session::body_boolean(const BooleanTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& base = body_for(doc, target.base);

    if (target.tool.empty()) {
        throw Error{"missing-tool", "a boolean needs at least one --tool body"};
    }
    const std::vector<PartDesign::Body*> tools = bodies_for(doc, target.tool, base);

    const char* type = target.operation == "union"       ? "Fuse"
                       : target.operation == "cut"         ? "Cut"
                       : target.operation == "intersect"   ? "Common"
                                                            : nullptr;
    if (type == nullptr) {
        throw Error{"invalid-operation",
                    "operation must be union, cut or intersect, not " + target.operation};
    }

    const std::string wanted = target.name.empty() ? std::string("Boolean") : target.name;
    auto* feature =
        static_cast<PartDesign::Boolean*>(doc.addObject("PartDesign::Boolean", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"boolean-failed", "FreeCAD refused to create a PartDesign::Boolean"};
    }

    feature->Type.setValue(type);
    App::DocumentObject* previous_tip = base.Tip.getValue();
    base.addObject(feature);
    feature->addObjects(std::vector<App::DocumentObject*>(tools.begin(), tools.end()));
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, base, previous_tip, *feature,
                                                    /*additive=*/type == "Fuse",
                                                    /*check_manifold=*/type == "Fuse");

    // FreeCAD's own recompute does not treat an empty result as an error - an
    // intersect of disjoint bodies produces a valid, empty compound - so a
    // boolean needs its own emptiness check on top of `recompute_or_rollback`,
    // with the same rollback it would have applied on a hard failure.
    if (!has_solid(feature->Shape.getValue())) {
        base.Tip.setValue(previous_tip);
        base.removeObject(feature);
        doc.removeObject(feature->getNameInDocument());
        (void)recompute_document(doc);
        throw Error{"empty-result",
                    "the " + target.operation + " of " + base.getNameInDocument() +
                        " left no solid - are the bodies disjoint?"};
    }

    mark_changed(doc.getName());
    gui::fit_view();

    json::Value tool_echo = json::Value::array();
    for (PartDesign::Body* tool : tools) {
        tool_echo.push(json::Value::string(tool->getNameInDocument()));
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(base.getNameInDocument()));
    out.set("boolean", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("operation", json::Value::string(target.operation));
    out.set("tool", std::move(tool_echo));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!base.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(base));
    out.set("shape", shape_of(base));
    return out;
}

json::Value Session::new_sketch(const SketchTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);

    App::Origin* origin = body.getOrigin();
    if (origin == nullptr) {
        throw Error{"no-origin", std::string("body ") + body.getNameInDocument() +
                                     " has no origin planes"};
    }

    const std::string wanted_plane = target.plane.empty() ? std::string("xy") : target.plane;
    App::Plane* datum = nullptr;
    if (wanted_plane == "xy") {
        datum = origin->getXY();
    }
    else if (wanted_plane == "xz") {
        datum = origin->getXZ();
    }
    else if (wanted_plane == "yz") {
        datum = origin->getYZ();
    }
    else {
        throw Error{"unknown-plane", "plane must be xy, xz or yz, got " + wanted_plane};
    }
    if (datum == nullptr) {
        throw Error{"no-origin", "origin plane " + wanted_plane + " is missing"};
    }

    if (!target.name.empty()) {
        require_document_identifier(target.name);
    }
    const std::string wanted = target.name.empty() ? std::string("Sketch") : target.name;
    auto* sketch = static_cast<Sketcher::SketchObject*>(
        doc.addObject("Sketcher::SketchObject", wanted.c_str()));
    if (sketch == nullptr) {
        throw Error{"sketch-failed", "FreeCAD refused to create a Sketcher::SketchObject"};
    }

    sketch->AttachmentSupport.setValue(datum, "");
    sketch->MapMode.setValue("FlatFace");
    // The offset is read in the plane's own axes: x and y slide the sketch
    // inside the plane, z lifts it off, and the rotation spins it about its
    // own normal. That is what makes a body placeable without a second body.
    const double x = params::resolve(doc, target.offset_x);
    const double y = params::resolve(doc, target.offset_y);
    const double z = params::resolve(doc, target.offset_z);
    const double turn = params::resolve(doc, target.rotate);
    sketch->AttachmentOffset.setValue(
        Base::Placement(Base::Vector3d(x, y, z),
                        Base::Rotation(Base::Vector3d(0.0, 0.0, 1.0), turn * M_PI / 180.0)));
    body.addObject(sketch);

    for (const auto& [slot, value] :
         {std::pair{"offset_x", target.offset_x}, std::pair{"offset_y", target.offset_y},
          std::pair{"offset_z", target.offset_z}, std::pair{"rotate", target.rotate}}) {
        params::apply(doc, *sketch, sketch_slot_path(slot), value);
    }

    // The attachment engine only runs on execute, and the reported basis is
    // worthless until it has.
    sketch->recomputeFeature();
    mark_changed(doc.getName());

    json::Value offset = json::Value::object();
    offset.set("x", params::slot_json(*sketch, "AttachmentOffset.Base.x", measure(x)));
    offset.set("y", params::slot_json(*sketch, "AttachmentOffset.Base.y", measure(y)));
    offset.set("z", params::slot_json(*sketch, "AttachmentOffset.Base.z", measure(z)));
    offset.set("rotate",
               params::slot_json(*sketch, "AttachmentOffset.Rotation.Angle", measure(turn)));

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("sketch", json::Value::string(sketch->getNameInDocument()));
    out.set("label", json::Value::string(sketch->Label.getValue()));
    out.set("plane", json::Value::string(datum->getNameInDocument()));
    out.set("offset", std::move(offset));
    out.set("basis", basis_of(*sketch));
    return out;
}

json::Value Session::rectangle(const RectangleTarget& target)
{
    App::Document& doc = document_for(target.document);
    const double width = params::resolve(doc, target.width);
    const double height = params::resolve(doc, target.height);
    const double x = params::resolve(doc, target.x);
    const double y = params::resolve(doc, target.y);

    if (!(width > 0.0) || !(height > 0.0)) {
        throw Error{"invalid-dimension", "width and height must be positive"};
    }

    Sketcher::SketchObject& sketch = sketch_for(doc, target.sketch);
    const int base = static_cast<int>(sketch.getInternalGeometry().size());
    const std::string n_width = unique_name(sketch, "width");
    const std::string n_height = unique_name(sketch, "height");
    const std::string n_x = unique_name(sketch, "x");
    const std::string n_y = unique_name(sketch, "y");

    const double left = target.centered ? x - width * 0.5 : x;
    const double bottom = target.centered ? y - height * 0.5 : y;

    const Base::Vector3d corners[4] = {Base::Vector3d(left, bottom, 0.0),
                                       Base::Vector3d(left + width, bottom, 0.0),
                                       Base::Vector3d(left + width, bottom + height, 0.0),
                                       Base::Vector3d(left, bottom + height, 0.0)};
    for (int i = 0; i < 4; ++i) {
        Part::GeomLineSegment line;
        line.setPoints(corners[i], corners[(i + 1) % 4]);
        sketch.addGeometry(&line, false);
    }

    auto constrain = [&sketch](Sketcher::ConstraintType type,
                               int first,
                               PointPos first_pos,
                               int second,
                               PointPos second_pos,
                               double value) {
        Sketcher::Constraint constraint;
        constraint.Type = type;
        constraint.First = first;
        constraint.FirstPos = first_pos;
        constraint.Second = second;
        constraint.SecondPos = second_pos;
        constraint.setValue(value);
        return sketch.addConstraint(&constraint);
    };

    for (int i = 0; i < 4; ++i) {
        constrain(Sketcher::Coincident, base + i, PointPos::end, base + (i + 1) % 4,
                  PointPos::start, 0.0);
    }
    constrain(Sketcher::Horizontal, base + 0, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Horizontal, base + 2, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, base + 1, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, base + 3, PointPos::none, kGeoUndefined, PointPos::none, 0.0);

    if (target.centered) {
        // Pinning a corner would centre the rectangle only until someone drives
        // the width; a construction point the diagonal is symmetric about keeps
        // it centred through every later `param set`.
        Part::GeomPoint anchor(Base::Vector3d(x, y, 0.0));
        const int spot = sketch.addGeometry(&anchor, true);
        pin_to_origin(sketch, spot, PointPos::start, x, y, n_x, n_y);

        Sketcher::Constraint symmetric;
        symmetric.Type = Sketcher::Symmetric;
        symmetric.First = base + 0;
        symmetric.FirstPos = PointPos::start;
        symmetric.Second = base + 2;
        symmetric.SecondPos = PointPos::start;
        symmetric.Third = spot;
        symmetric.ThirdPos = PointPos::start;
        sketch.addConstraint(&symmetric);
    }
    else {
        pin_to_origin(sketch, base + 0, PointPos::start, left, bottom, n_x, n_y);
    }

    name_constraint(sketch,
                    constrain(Sketcher::DistanceX, base + 0, PointPos::start, base + 0,
                              PointPos::end, width),
                    n_width);
    name_constraint(sketch,
                    constrain(Sketcher::DistanceY, base + 1, PointPos::start, base + 1,
                              PointPos::end, height),
                    n_height);

    for (const auto& [slot, value] :
         {std::pair{n_width, target.width}, std::pair{n_height, target.height},
          std::pair{n_x, target.x}, std::pair{n_y, target.y}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("width", params::slot_json(sketch, "Constraints." + n_width, measure(width)));
    out.set("height", params::slot_json(sketch, "Constraints." + n_height, measure(height)));
    out.set("centered", json::Value::boolean(target.centered));
    out.set("corner", point(Base::Vector3d(left, bottom, 0.0)));
    out.set("slots", slot_names({{"width", n_width}, {"height", n_height},
                                 {"x", n_x}, {"y", n_y}}));
    append_sketch_detail(out, sketch);
    return out;
}

json::Value Session::circle(const CircleTarget& target)
{
    App::Document& doc = document_for(target.document);
    const double radius = params::resolve(doc, target.radius);
    const double x = params::resolve(doc, target.x);
    const double y = params::resolve(doc, target.y);

    if (!(radius > 0.0)) {
        throw Error{"invalid-dimension", "radius must be positive"};
    }

    Sketcher::SketchObject& sketch = sketch_for(doc, target.sketch);
    const int geo = static_cast<int>(sketch.getInternalGeometry().size());
    const std::string n_radius = unique_name(sketch, "radius");
    const std::string n_x = unique_name(sketch, "x");
    const std::string n_y = unique_name(sketch, "y");

    Part::GeomCircle geometry;
    geometry.setLocation(Base::Vector3d(x, y, 0.0));
    geometry.setRadius(radius);
    sketch.addGeometry(&geometry, false);

    Sketcher::Constraint dimension;
    dimension.Type = Sketcher::Radius;
    dimension.First = geo;
    dimension.FirstPos = PointPos::none;
    dimension.setValue(radius);
    name_constraint(sketch, sketch.addConstraint(&dimension), n_radius);

    pin_to_origin(sketch, geo, PointPos::mid, x, y, n_x, n_y);

    for (const auto& [slot, value] : {std::pair{n_radius, target.radius},
                                      std::pair{n_x, target.x}, std::pair{n_y, target.y}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("radius", params::slot_json(sketch, "Constraints." + n_radius, measure(radius)));
    out.set("centre", point(Base::Vector3d(x, y, 0.0)));
    out.set("slots", slot_names({{"radius", n_radius}, {"x", n_x}, {"y", n_y}}));
    append_sketch_detail(out, sketch);
    return out;
}

json::Value Session::line(const LineTarget& target)
{
    App::Document& doc = document_for(target.document);
    const double x1 = params::resolve(doc, target.x1);
    const double y1 = params::resolve(doc, target.y1);
    const double x2 = params::resolve(doc, target.x2);
    const double y2 = params::resolve(doc, target.y2);

    if (x1 == x2 && y1 == y2) {
        throw Error{"invalid-dimension", "a line needs two distinct endpoints"};
    }

    Sketcher::SketchObject& sketch = sketch_for(doc, target.sketch);
    const int geo = static_cast<int>(sketch.getInternalGeometry().size());
    const std::string n_x1 = unique_name(sketch, "x1");
    const std::string n_y1 = unique_name(sketch, "y1");
    const std::string n_x2 = unique_name(sketch, "x2");
    const std::string n_y2 = unique_name(sketch, "y2");

    Part::GeomLineSegment segment;
    segment.setPoints(Base::Vector3d(x1, y1, 0.0), Base::Vector3d(x2, y2, 0.0));
    sketch.addGeometry(&segment, false);

    pin_to_origin(sketch, geo, PointPos::start, x1, y1, n_x1, n_y1);
    pin_to_origin(sketch, geo, PointPos::end, x2, y2, n_x2, n_y2);

    for (const auto& [slot, value] :
         {std::pair{n_x1, target.x1}, std::pair{n_y1, target.y1},
          std::pair{n_x2, target.x2}, std::pair{n_y2, target.y2}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("start", point(Base::Vector3d(x1, y1, 0.0)));
    out.set("end", point(Base::Vector3d(x2, y2, 0.0)));
    out.set("slots", slot_names({{"x1", n_x1}, {"y1", n_y1}, {"x2", n_x2}, {"y2", n_y2}}));
    append_sketch_detail(out, sketch);
    return out;
}

json::Value Session::arc(const ArcTarget& target)
{
    App::Document& doc = document_for(target.document);
    const double x1 = params::resolve(doc, target.x1);
    const double y1 = params::resolve(doc, target.y1);
    const double x2 = params::resolve(doc, target.x2);
    const double y2 = params::resolve(doc, target.y2);
    const double radius = params::resolve(doc, target.radius);

    if (!(radius > 0.0)) {
        throw Error{"invalid-dimension", "radius must be positive"};
    }
    const double chord = std::hypot(x2 - x1, y2 - y1);
    if (!(chord > 0.0)) {
        throw Error{"invalid-dimension", "an arc needs two distinct endpoints"};
    }
    if (chord > 2.0 * radius) {
        throw Error{"invalid-dimension",
                    "endpoints are " + std::to_string(chord) +
                        " mm apart, more than a radius of " + std::to_string(radius) +
                        " can span; the radius must be at least " + std::to_string(chord / 2.0)};
    }

    // The centre sits on the chord's perpendicular bisector. Going CCW from
    // (x1,y1), the minor arc's centre is on the left of the chord; `large`
    // flips it to the right and the arc goes the long way round.
    const double half = chord * 0.5;
    const double lift = std::sqrt(std::max(0.0, radius * radius - half * half));
    const double ux = (x2 - x1) / chord;
    const double uy = (y2 - y1) / chord;
    const double side = target.large ? -1.0 : 1.0;
    const double cx = (x1 + x2) * 0.5 + side * lift * -uy;
    const double cy = (y1 + y2) * 0.5 + side * lift * ux;

    Sketcher::SketchObject& sketch = sketch_for(doc, target.sketch);
    const int geo = static_cast<int>(sketch.getInternalGeometry().size());
    const std::string n_x1 = unique_name(sketch, "x1");
    const std::string n_y1 = unique_name(sketch, "y1");
    const std::string n_x2 = unique_name(sketch, "x2");
    const std::string n_y2 = unique_name(sketch, "y2");
    const std::string n_radius = unique_name(sketch, "radius");

    Part::GeomArcOfCircle geometry;
    geometry.setLocation(Base::Vector3d(cx, cy, 0.0));
    geometry.setRadius(radius);
    geometry.setRange(std::atan2(y1 - cy, x1 - cx), std::atan2(y2 - cy, x2 - cx), true);
    sketch.addGeometry(&geometry, false);

    Sketcher::Constraint dimension;
    dimension.Type = Sketcher::Radius;
    dimension.First = geo;
    dimension.FirstPos = PointPos::none;
    dimension.setValue(radius);
    name_constraint(sketch, sketch.addConstraint(&dimension), n_radius);

    pin_to_origin(sketch, geo, PointPos::start, x1, y1, n_x1, n_y1);
    pin_to_origin(sketch, geo, PointPos::end, x2, y2, n_x2, n_y2);

    for (const auto& [slot, value] :
         {std::pair{n_x1, target.x1}, std::pair{n_y1, target.y1},
          std::pair{n_x2, target.x2}, std::pair{n_y2, target.y2},
          std::pair{n_radius, target.radius}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("start", point(Base::Vector3d(x1, y1, 0.0)));
    out.set("end", point(Base::Vector3d(x2, y2, 0.0)));
    out.set("centre", point(Base::Vector3d(cx, cy, 0.0)));
    out.set("radius", params::slot_json(sketch, "Constraints." + n_radius, measure(radius)));
    out.set("large", json::Value::boolean(target.large));
    out.set("slots", slot_names({{"x1", n_x1}, {"y1", n_y1}, {"x2", n_x2}, {"y2", n_y2},
                                 {"radius", n_radius}}));
    append_sketch_detail(out, sketch);
    return out;
}

json::Value Session::polyline(const PolylineTarget& target)
{
    App::Document& doc = document_for(target.document);

    const std::size_t count = target.points.size();
    if (count < 2 || (target.close && count < 3)) {
        throw Error{"invalid-dimension",
                    target.close ? "a closed polyline needs at least 3 points"
                                 : "a polyline needs at least 2 points"};
    }

    std::vector<Base::Vector3d> vertices;
    vertices.reserve(count);
    for (const auto& [sx, sy] : target.points) {
        vertices.emplace_back(params::resolve(doc, sx), params::resolve(doc, sy), 0.0);
    }
    const std::size_t segments = target.close ? count : count - 1;
    for (std::size_t i = 0; i < segments; ++i) {
        const Base::Vector3d& from = vertices[i];
        const Base::Vector3d& to = vertices[(i + 1) % count];
        if (from.x == to.x && from.y == to.y) {
            throw Error{"invalid-dimension",
                        "points " + std::to_string(i + 1) + " and " +
                            std::to_string((i + 1) % count + 1) + " coincide"};
        }
    }

    Sketcher::SketchObject& sketch = sketch_for(doc, target.sketch);
    const int geo = static_cast<int>(sketch.getInternalGeometry().size());

    std::vector<std::string> names_x;
    std::vector<std::string> names_y;
    for (std::size_t i = 0; i < count; ++i) {
        names_x.push_back(unique_name(sketch, "x" + std::to_string(i + 1)));
        names_y.push_back(unique_name(sketch, "y" + std::to_string(i + 1)));
    }

    for (std::size_t i = 0; i < segments; ++i) {
        Part::GeomLineSegment segment;
        segment.setPoints(vertices[i], vertices[(i + 1) % count]);
        sketch.addGeometry(&segment, false);
    }

    const std::size_t joints = target.close ? segments : segments - 1;
    for (std::size_t i = 0; i < joints; ++i) {
        Sketcher::Constraint joint;
        joint.Type = Sketcher::Coincident;
        joint.First = geo + static_cast<int>(i);
        joint.FirstPos = PointPos::end;
        joint.Second = geo + static_cast<int>((i + 1) % segments);
        joint.SecondPos = PointPos::start;
        sketch.addConstraint(&joint);
    }

    // Vertex i is the start of segment i; an open polyline's last vertex only
    // exists as the end of the segment before it.
    for (std::size_t i = 0; i < count; ++i) {
        const bool tail = i == segments;
        pin_to_origin(sketch, geo + static_cast<int>(tail ? i - 1 : i),
                      tail ? PointPos::end : PointPos::start, vertices[i].x, vertices[i].y,
                      names_x[i], names_y[i]);
    }

    for (std::size_t i = 0; i < count; ++i) {
        params::apply(doc, sketch, sketch_slot_path(names_x[i]), target.points[i].first);
        params::apply(doc, sketch, sketch_slot_path(names_y[i]), target.points[i].second);
    }

    mark_changed(doc.getName());

    json::Value points = json::Value::array();
    json::Value slots = json::Value::object();
    for (std::size_t i = 0; i < count; ++i) {
        points.push(point(vertices[i]));
        slots.set("x" + std::to_string(i + 1), json::Value::string(names_x[i]));
        slots.set("y" + std::to_string(i + 1), json::Value::string(names_y[i]));
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("points", std::move(points));
    out.set("closed", json::Value::boolean(target.close));
    out.set("slots", std::move(slots));
    append_sketch_detail(out, sketch);
    return out;
}

namespace {

/// Everything a pad and a pocket share: resolve the body and profile, refuse a
/// profile that belongs elsewhere, then recompute and report the solid.
struct ExtrudeParts
{
    App::Document* doc = nullptr;
    PartDesign::Body* body = nullptr;
    Sketcher::SketchObject* profile = nullptr;
};

/// The same resolution pad, pocket, revolve and groove all need: a body, and
/// the sketch inside it a feature is about to consume.
ExtrudeParts resolve_profile(const std::string& document,
                             const std::string& body,
                             const std::string& sketch)
{
    ExtrudeParts parts;
    parts.doc = &document_for(document);
    parts.body = &body_for(*parts.doc, body);
    parts.profile = &sketch_for(*parts.doc, sketch, parts.body);

    if (parts.profile->getInternalGeometry().empty()) {
        throw Error{"empty-sketch",
                    std::string("sketch ") + parts.profile->getNameInDocument() +
                        " has no geometry"};
    }
    if (PartDesign::Body::findBodyOf(parts.profile) != parts.body) {
        throw Error{"foreign-sketch",
                    std::string("sketch ") + parts.profile->getNameInDocument() +
                        " does not belong to " + parts.body->getNameInDocument()};
    }
    return parts;
}

ExtrudeParts resolve_extrude(const ExtrudeTarget& target)
{
    return resolve_profile(target.document, target.body, target.sketch);
}

/// The sketch's own local axis, the only one every profile has for free
/// without naming an edge: "x" is its H_Axis, "y" its V_Axis.
void set_revolve_axis(App::PropertyLinkSub& reference_axis,
                      Sketcher::SketchObject& sketch,
                      const std::string& axis)
{
    if (axis == "x") {
        reference_axis.setValue(&sketch, {"H_Axis"});
    }
    else if (axis == "y") {
        reference_axis.setValue(&sketch, {"V_Axis"});
    }
    else {
        throw Error{"invalid-param", "axis must be x or y, got " + axis};
    }
}

/// A body plus every sketch a loft threads through, in the caller's order -
/// the first is Profile, the rest are Sections.
struct LoftParts
{
    App::Document* doc = nullptr;
    PartDesign::Body* body = nullptr;
    std::vector<Sketcher::SketchObject*> sketches;
};

/// True once some `ProfileBased` feature already threads through `sketch`,
/// as its `Profile` or - for an existing loft - one of its `Sections`. The
/// same sketch feeding two features at once is never what a loft call means.
bool sketch_consumed(App::Document& doc, App::DocumentObject* sketch)
{
    for (PartDesign::ProfileBased* feature : doc.getObjectsOfType<PartDesign::ProfileBased>()) {
        if (feature->Profile.getValue() == sketch) {
            return true;
        }
        if (auto* loft = dynamic_cast<PartDesign::Loft*>(feature)) {
            for (App::DocumentObject* section : loft->Sections.getValues()) {
                if (section == sketch) {
                    return true;
                }
            }
        }
    }
    return false;
}

struct SketchPlane
{
    Base::Vector3d origin;
    Base::Vector3d normal;
};

SketchPlane plane_of(const Sketcher::SketchObject& sketch)
{
    const Base::Placement placement = sketch.Placement.getValue();
    return {placement.getPosition(),
            placement.getRotation().multVec(Base::Vector3d(0.0, 0.0, 1.0))};
}

/// Same plane, either normal direction: parallel normals and no offset
/// between the origins along that normal.
bool coplanar(const SketchPlane& a, const SketchPlane& b)
{
    const Base::Vector3d na = a.normal.Normalized();
    const Base::Vector3d nb = b.normal.Normalized();
    if (std::abs(na.Dot(nb)) < 1.0 - 1e-6) {
        return false;
    }
    return std::abs((b.origin - a.origin).Dot(na)) < kSelectionTolerance;
}

/// The body and its resolved sketches a loft needs: at least two, each
/// belonging to `body`, none already consumed by another feature, and no two
/// of them coplanar - the same three refusals a degenerate loft would
/// otherwise fail on deep inside OCCT, with no clear reason attached.
LoftParts resolve_loft(const LoftTarget& target)
{
    LoftParts parts;
    parts.doc = &document_for(target.document);
    parts.body = &body_for(*parts.doc, target.body);

    if (target.sketches.size() < 2) {
        throw Error{"invalid-loft",
                    "loft needs at least two sketches, got " +
                        std::to_string(target.sketches.size())};
    }

    for (const std::string& name : target.sketches) {
        Sketcher::SketchObject& sketch = sketch_for(*parts.doc, name, parts.body);
        if (PartDesign::Body::findBodyOf(&sketch) != parts.body) {
            throw Error{"foreign-sketch",
                        std::string("sketch ") + sketch.getNameInDocument() +
                            " does not belong to " + parts.body->getNameInDocument()};
        }
        if (sketch_consumed(*parts.doc, &sketch)) {
            throw Error{"sketch-consumed",
                        std::string("sketch ") + sketch.getNameInDocument() +
                            " is already used by another feature"};
        }
        parts.sketches.push_back(&sketch);
    }

    for (std::size_t i = 0; i < parts.sketches.size(); ++i) {
        for (std::size_t j = i + 1; j < parts.sketches.size(); ++j) {
            if (coplanar(plane_of(*parts.sketches[i]), plane_of(*parts.sketches[j]))) {
                throw Error{"coplanar-sketches",
                            std::string("sketch ") + parts.sketches[i]->getNameInDocument() +
                                " and " + parts.sketches[j]->getNameInDocument() +
                                " lie on the same plane"};
            }
        }
    }

    return parts;
}

json::Value echo_sketches(const std::vector<Sketcher::SketchObject*>& sketches)
{
    json::Value out = json::Value::array();
    for (Sketcher::SketchObject* sketch : sketches) {
        out.push(json::Value::string(sketch->getNameInDocument()));
    }
    return out;
}

}  // namespace

json::Value Session::pad(const ExtrudeTarget& target)
{
    const ExtrudeParts parts = resolve_extrude(target);
    App::Document& doc = *parts.doc;

    const double length = params::resolve(doc, target.length);
    if (!(length > 0.0)) {
        throw Error{"invalid-dimension", "length must be positive"};
    }

    const std::string wanted = target.name.empty() ? std::string("Pad") : target.name;
    auto* feature = static_cast<PartDesign::Pad*>(doc.addObject("PartDesign::Pad", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"pad-failed", "FreeCAD refused to create a PartDesign::Pad"};
    }

    feature->Profile.setValue(parts.profile, std::vector<std::string>());
    feature->Length.setValue(length);
    params::apply(doc, *feature, "Length", target.length);
    feature->TaperAngle.setValue(params::resolve(doc, target.taper));
    params::apply(doc, *feature, "TaperAngle", target.taper);
    // Midplane is deprecated in 1.1 and only forwards to SideType with a warning.
    feature->SideType.setValue(target.midplane ? "Symmetric" : "One side");
    feature->Reversed.setValue(target.reversed);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    // What the GUI does after a pad: the profile is consumed by the solid, so
    // leaving it visible only draws edges inside the part.
    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature,
                                                    /*additive=*/false, /*check_manifold=*/true);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("pad", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("length",
            params::slot_json(*feature, "Length", measure(feature->Length.getValue())));
    out.set("taper",
            params::slot_json(*feature, "TaperAngle", measure(feature->TaperAngle.getValue())));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

json::Value Session::pocket(const ExtrudeTarget& target)
{
    const ExtrudeParts parts = resolve_extrude(target);
    App::Document& doc = *parts.doc;

    const double length = params::resolve(doc, target.length);
    if (!target.through_all && !(length > 0.0)) {
        throw Error{"invalid-dimension", "length must be positive unless the pocket is through all"};
    }

    if (parts.body->Shape.getValue().IsNull()) {
        throw Error{"no-material",
                    std::string("body ") + parts.body->getNameInDocument() +
                        " has nothing to cut, pad something first"};
    }

    const std::string wanted = target.name.empty() ? std::string("Pocket") : target.name;
    auto* feature =
        static_cast<PartDesign::Pocket*>(doc.addObject("PartDesign::Pocket", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"pocket-failed", "FreeCAD refused to create a PartDesign::Pocket"};
    }

    feature->Profile.setValue(parts.profile, std::vector<std::string>());
    feature->Type.setValue(target.through_all ? "ThroughAll" : "Length");
    if (!target.through_all) {
        feature->Length.setValue(length);
        params::apply(doc, *feature, "Length", target.length);
    }
    feature->TaperAngle.setValue(params::resolve(doc, target.taper));
    params::apply(doc, *feature, "TaperAngle", target.taper);
    feature->SideType.setValue(target.midplane ? "Symmetric" : "One side");
    feature->Reversed.setValue(target.reversed);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("pocket", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("length",
            params::slot_json(*feature, "Length", measure(feature->Length.getValue())));
    out.set("taper",
            params::slot_json(*feature, "TaperAngle", measure(feature->TaperAngle.getValue())));
    out.set("through_all", json::Value::boolean(target.through_all));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

json::Value Session::revolve(const RevolveTarget& target)
{
    const ExtrudeParts parts = resolve_profile(target.document, target.body, target.sketch);
    App::Document& doc = *parts.doc;

    const double angle = params::resolve(doc, target.angle);
    if (!(angle > 0.0 && angle <= 360.0)) {
        throw Error{"invalid-dimension", "angle must be between 0 and 360 degrees"};
    }

    const std::string wanted = target.name.empty() ? std::string("Revolution") : target.name;
    auto* feature = static_cast<PartDesign::Revolution*>(
        doc.addObject("PartDesign::Revolution", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"revolve-failed", "FreeCAD refused to create a PartDesign::Revolution"};
    }

    feature->Profile.setValue(parts.profile, std::vector<std::string>());
    set_revolve_axis(feature->ReferenceAxis, *parts.profile, target.axis);
    feature->Angle.setValue(angle);
    params::apply(doc, *feature, "Angle", target.angle);
    feature->Midplane.setValue(target.midplane);
    feature->Reversed.setValue(target.reversed);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature,
                                                    /*additive=*/false, /*check_manifold=*/true);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("revolve", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("angle", params::slot_json(*feature, "Angle", measure(feature->Angle.getValue())));
    out.set("axis", json::Value::string(target.axis));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

json::Value Session::groove(const RevolveTarget& target)
{
    const ExtrudeParts parts = resolve_profile(target.document, target.body, target.sketch);
    App::Document& doc = *parts.doc;

    const double angle = params::resolve(doc, target.angle);
    if (!(angle > 0.0 && angle <= 360.0)) {
        throw Error{"invalid-dimension", "angle must be between 0 and 360 degrees"};
    }

    if (parts.body->Shape.getValue().IsNull()) {
        throw Error{"no-material",
                    std::string("body ") + parts.body->getNameInDocument() +
                        " has nothing to cut, pad or revolve something first"};
    }

    const std::string wanted = target.name.empty() ? std::string("Groove") : target.name;
    auto* feature =
        static_cast<PartDesign::Groove*>(doc.addObject("PartDesign::Groove", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"groove-failed", "FreeCAD refused to create a PartDesign::Groove"};
    }

    feature->Profile.setValue(parts.profile, std::vector<std::string>());
    set_revolve_axis(feature->ReferenceAxis, *parts.profile, target.axis);
    feature->Angle.setValue(angle);
    params::apply(doc, *feature, "Angle", target.angle);
    feature->Midplane.setValue(target.midplane);
    feature->Reversed.setValue(target.reversed);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("groove", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("angle", params::slot_json(*feature, "Angle", measure(feature->Angle.getValue())));
    out.set("axis", json::Value::string(target.axis));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

json::Value Session::loft_new(const LoftTarget& target)
{
    const LoftParts parts = resolve_loft(target);
    App::Document& doc = *parts.doc;

    const std::string wanted = target.name.empty() ? std::string("Loft") : target.name;
    auto* feature = static_cast<PartDesign::AdditiveLoft*>(
        doc.addObject("PartDesign::AdditiveLoft", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"loft-failed", "FreeCAD refused to create a PartDesign::AdditiveLoft"};
    }

    feature->Profile.setValue(parts.sketches.front(), std::vector<std::string>());
    const std::vector<App::DocumentObject*> sections(parts.sketches.begin() + 1,
                                                       parts.sketches.end());
    feature->Sections.setValues(sections, std::vector<std::string>(sections.size(), std::string()));
    feature->Ruled.setValue(target.ruled);
    feature->Closed.setValue(target.closed);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    for (Sketcher::SketchObject* sketch : parts.sketches) {
        sketch->Visibility.setValue(false);
    }
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature,
                                                    /*additive=*/false, /*check_manifold=*/true);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketches", echo_sketches(parts.sketches));
    out.set("loft", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("ruled", json::Value::boolean(target.ruled));
    out.set("closed", json::Value::boolean(target.closed));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

json::Value Session::loft_pocket(const LoftTarget& target)
{
    const LoftParts parts = resolve_loft(target);
    App::Document& doc = *parts.doc;

    if (parts.body->Shape.getValue().IsNull()) {
        throw Error{"no-material",
                    std::string("body ") + parts.body->getNameInDocument() +
                        " has nothing to cut, pad something first"};
    }

    const std::string wanted = target.name.empty() ? std::string("Loft") : target.name;
    auto* feature = static_cast<PartDesign::SubtractiveLoft*>(
        doc.addObject("PartDesign::SubtractiveLoft", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"loft-failed", "FreeCAD refused to create a PartDesign::SubtractiveLoft"};
    }

    feature->Profile.setValue(parts.sketches.front(), std::vector<std::string>());
    const std::vector<App::DocumentObject*> sections(parts.sketches.begin() + 1,
                                                       parts.sketches.end());
    feature->Sections.setValues(sections, std::vector<std::string>(sections.size(), std::string()));
    feature->Ruled.setValue(target.ruled);
    App::DocumentObject* previous_tip = parts.body->Tip.getValue();
    parts.body->addObject(feature);

    for (Sketcher::SketchObject* sketch : parts.sketches) {
        sketch->Visibility.setValue(false);
    }
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, *parts.body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketches", echo_sketches(parts.sketches));
    out.set("loft", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("ruled", json::Value::boolean(target.ruled));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(*parts.body));
    out.set("shape", shape_of(*parts.body));
    return out;
}

namespace {

/// The origin every mirror and pattern resolves its plane or axis against -
/// the body's own, global and independent of any sketch, unlike revolve's
/// H_Axis/V_Axis which only exist inside a profile.
App::Origin& origin_of(PartDesign::Body& body)
{
    App::Origin* origin = body.getOrigin();
    if (origin == nullptr) {
        throw Error{"no-origin",
                    std::string("body ") + body.getNameInDocument() + " has no origin planes"};
    }
    return *origin;
}

App::Plane& plane_of(App::Origin& origin, const std::string& name)
{
    App::Plane* datum = nullptr;
    if (name == "xy") {
        datum = origin.getXY();
    }
    else if (name == "xz") {
        datum = origin.getXZ();
    }
    else if (name == "yz") {
        datum = origin.getYZ();
    }
    else {
        throw Error{"unknown-plane", "plane must be xy, xz or yz, got " + name};
    }
    if (datum == nullptr) {
        throw Error{"no-origin", "origin plane " + name + " is missing"};
    }
    return *datum;
}

App::Line& axis_of(App::Origin& origin, const std::string& name)
{
    App::Line* line = nullptr;
    if (name == "x") {
        line = origin.getX();
    }
    else if (name == "y") {
        line = origin.getY();
    }
    else if (name == "z") {
        line = origin.getZ();
    }
    else {
        throw Error{"unknown-axis", "axis must be x, y or z, got " + name};
    }
    if (line == nullptr) {
        throw Error{"no-origin", "origin axis " + name + " is missing"};
    }
    return *line;
}

}  // namespace

json::Value Session::mirror(const MirrorTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);
    App::Plane& datum = plane_of(origin_of(body), target.plane.empty() ? "xy" : target.plane);
    const std::vector<PartDesign::Feature*> originals = features_for(doc, target.features, body);

    const std::string wanted = target.name.empty() ? std::string("Mirrored") : target.name;
    auto* feature =
        static_cast<PartDesign::Mirrored*>(doc.addObject("PartDesign::Mirrored", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"mirror-failed", "FreeCAD refused to create a PartDesign::Mirrored"};
    }

    feature->Originals.setValues(std::vector<App::DocumentObject*>(originals.begin(), originals.end()));
    feature->MirrorPlane.setValue(&datum, {""});
    App::DocumentObject* previous_tip = body.Tip.getValue();
    body.addObject(feature);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value features_echo = json::Value::array();
    for (PartDesign::Feature* original : originals) {
        features_echo.push(json::Value::string(original->getNameInDocument()));
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("mirror", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("plane", json::Value::string(datum.getNameInDocument()));
    out.set("features", std::move(features_echo));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!body.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(body));
    out.set("shape", shape_of(body));
    return out;
}

json::Value Session::pattern_linear(const LinearPatternTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);
    App::Line& direction = axis_of(origin_of(body), target.direction);
    const std::vector<PartDesign::Feature*> originals = features_for(doc, target.features, body);

    const double spacing = params::resolve(doc, target.spacing);
    if (spacing == 0.0) {
        throw Error{"invalid-dimension", "spacing must not be zero"};
    }
    if (target.count < 2) {
        throw Error{"invalid-dimension", "count must be at least 2"};
    }

    const std::string wanted = target.name.empty() ? std::string("LinearPattern") : target.name;
    auto* feature = static_cast<PartDesign::LinearPattern*>(
        doc.addObject("PartDesign::LinearPattern", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"pattern-failed", "FreeCAD refused to create a PartDesign::LinearPattern"};
    }

    feature->Originals.setValues(std::vector<App::DocumentObject*>(originals.begin(), originals.end()));
    feature->Direction.setValue(&direction, {""});
    feature->Mode.setValue("Spacing");
    feature->Offset.setValue(spacing);
    params::apply(doc, *feature, "Offset", target.spacing);
    feature->Occurrences.setValue(target.count);
    feature->Reversed.setValue(target.reversed);
    App::DocumentObject* previous_tip = body.Tip.getValue();
    body.addObject(feature);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value features_echo = json::Value::array();
    for (PartDesign::Feature* original : originals) {
        features_echo.push(json::Value::string(original->getNameInDocument()));
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("pattern", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("direction", json::Value::string(target.direction));
    out.set("spacing",
            params::slot_json(*feature, "Offset", measure(feature->Offset.getValue())));
    out.set("count", json::Value::integer(feature->Occurrences.getValue()));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("features", std::move(features_echo));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!body.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(body));
    out.set("shape", shape_of(body));
    return out;
}

json::Value Session::pattern_polar(const PolarPatternTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);
    App::Line& axis = axis_of(origin_of(body), target.axis);
    const std::vector<PartDesign::Feature*> originals = features_for(doc, target.features, body);

    const double angle = params::resolve(doc, target.angle);
    if (!(angle > 0.0 && angle <= 360.0)) {
        throw Error{"invalid-dimension", "angle must be between 0 and 360 degrees"};
    }
    if (target.count < 2) {
        throw Error{"invalid-dimension", "count must be at least 2"};
    }

    const std::string wanted = target.name.empty() ? std::string("PolarPattern") : target.name;
    auto* feature = static_cast<PartDesign::PolarPattern*>(
        doc.addObject("PartDesign::PolarPattern", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"pattern-failed", "FreeCAD refused to create a PartDesign::PolarPattern"};
    }

    feature->Originals.setValues(std::vector<App::DocumentObject*>(originals.begin(), originals.end()));
    feature->Axis.setValue(&axis, {""});
    feature->Mode.setValue("Extent");
    feature->Angle.setValue(angle);
    params::apply(doc, *feature, "Angle", target.angle);
    feature->Occurrences.setValue(target.count);
    App::DocumentObject* previous_tip = body.Tip.getValue();
    body.addObject(feature);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value features_echo = json::Value::array();
    for (PartDesign::Feature* original : originals) {
        features_echo.push(json::Value::string(original->getNameInDocument()));
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("pattern", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("axis", json::Value::string(target.axis));
    out.set("angle", params::slot_json(*feature, "Angle", measure(feature->Angle.getValue())));
    out.set("count", json::Value::integer(feature->Occurrences.getValue()));
    out.set("features", std::move(features_echo));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!body.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(body));
    out.set("shape", shape_of(body));
    return out;
}

namespace {

/// Geometric edge selection for fillet/chamfer, so the request never carries a
/// raw `EdgeN` - FreeCAD's own edge names are the topological naming problem
/// and shift under any upstream change.
struct MatchedEdges
{
    std::vector<std::string> names;
    double total_length = 0.0;
    /// Whether every predicate was left at its default, i.e. this is the
    /// whole shape rather than a subset that happens to cover it. Drives
    /// `UseAllEdges`, which is what keeps a whole-shape selection valid
    /// across a later edit that changes the edge count.
    bool all_edges = true;
};

int axis_index(const std::string& axis)
{
    if (axis == "x") {
        return 0;
    }
    if (axis == "y") {
        return 1;
    }
    if (axis == "z") {
        return 2;
    }
    throw Error{"unknown-axis", "axis must be x, y or z, got " + axis};
}

gp_Dir axis_direction(const std::string& axis)
{
    switch (axis_index(axis)) {
        case 0: return {1.0, 0.0, 0.0};
        case 1: return {0.0, 1.0, 0.0};
        default: return {0.0, 0.0, 1.0};
    }
}

double length_of(const TopoDS_Edge& edge)
{
    GProp_GProps props;
    BRepGProp::LinearProperties(edge, props);
    return props.Mass();
}

bool matches(const TopoDS_Edge& edge, const EdgeSelection& selection, const Box& bounds)
{
    if (!selection.parallel.empty()) {
        BRepAdaptor_Curve curve(edge);
        if (curve.GetType() != GeomAbs_Line) {
            return false;
        }
        if (!curve.Line().Direction().IsParallel(axis_direction(selection.parallel),
                                                 Precision::Angular())) {
            return false;
        }
    }

    if (!selection.near_min.empty() || !selection.near_max.empty()) {
        const Box edge_box = box_of(edge);
        if (!edge_box.valid) {
            return false;
        }
        if (!selection.near_min.empty()) {
            const int axis = axis_index(selection.near_min);
            if (std::abs(edge_box.min[axis] - bounds.min[axis]) > kSelectionTolerance ||
                std::abs(edge_box.max[axis] - bounds.min[axis]) > kSelectionTolerance) {
                return false;
            }
        }
        if (!selection.near_max.empty()) {
            const int axis = axis_index(selection.near_max);
            if (std::abs(edge_box.min[axis] - bounds.max[axis]) > kSelectionTolerance ||
                std::abs(edge_box.max[axis] - bounds.max[axis]) > kSelectionTolerance) {
                return false;
            }
        }
    }

    const bool needs_length = selection.has_longer_than || selection.has_shorter_than;
    if (needs_length) {
        const double length = length_of(edge);
        if (selection.has_longer_than && !(length > selection.longer_than)) {
            return false;
        }
        if (selection.has_shorter_than && !(length < selection.shorter_than)) {
            return false;
        }
    }

    return true;
}

/// Resolves a selection to `EdgeN` names the way FreeCAD itself indexes them -
/// `TopExp::MapShapes`, not an ad-hoc walk - so the names match what
/// `Base.getSubValues()`/`getSubShape` expect. A selection that matches
/// nothing is a refusal, not a silent no-op dressup.
MatchedEdges match_edges(const TopoDS_Shape& shape, const EdgeSelection& selection)
{
    if (shape.IsNull()) {
        throw Error{"no-shape", "base feature has no shape to select edges from"};
    }

    TopTools_IndexedMapOfShape map;
    TopExp::MapShapes(shape, TopAbs_EDGE, map);
    const Box bounds = box_of(shape);

    const bool any_predicate = !selection.parallel.empty() || !selection.near_min.empty() ||
                               !selection.near_max.empty() || selection.has_longer_than ||
                               selection.has_shorter_than;

    MatchedEdges out;
    out.all_edges = !any_predicate;
    for (int index = 1; index <= map.Extent(); ++index) {
        const TopoDS_Edge edge = TopoDS::Edge(map.FindKey(index));
        if (any_predicate && !matches(edge, selection, bounds)) {
            continue;
        }
        out.names.push_back("Edge" + std::to_string(index));
        out.total_length += length_of(edge);
    }

    if (out.names.empty()) {
        throw Error{"no-edges-matched", "no edge of the base feature matched the selection"};
    }
    return out;
}

/// Splice a dressup in right after `base`, wherever `base` sits in the chain -
/// `body.addObject` always appends at the tip, which is wrong the moment
/// `base` is not the tip: `Body::insertObject` is what FreeCAD's own GUI uses
/// to dress an earlier feature, and it already reroutes whatever came after
/// `base` to build on the new feature instead, keeping `BaseFeature` in
/// agreement with `Base` (the edges' own feature). Only the Tip is left to the
/// caller, and only because it must move when `base` was the tip and must not
/// move otherwise - `body.removeObject` on rollback undoes both halves the
/// same way `insertObject` set them up.
void splice_dressup(PartDesign::Body& body, PartDesign::Feature& base, PartDesign::Feature& feature)
{
    App::DocumentObject* next = body.getNextSolidFeature(&base);
    body.insertObject(&feature, next, /*after=*/false);
    if (next == nullptr) {
        body.Tip.setValue(&feature);
    }
}

}  // namespace

json::Value Session::fillet(const FilletTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);
    const std::vector<PartDesign::Feature*> based = features_for(doc, target.features, body);
    if (based.size() > 1) {
        throw Error{"too-many-features", "fillet dresses one feature at a time, named " +
                                             std::to_string(based.size())};
    }
    PartDesign::Feature& base = *based.front();

    const double radius = params::resolve(doc, target.radius);
    if (radius <= 0.0) {
        throw Error{"invalid-dimension", "radius must be positive"};
    }

    const MatchedEdges matched = match_edges(base.Shape.getValue(), target.selection);

    const std::string wanted = target.name.empty() ? std::string("Fillet") : target.name;
    auto* feature =
        static_cast<PartDesign::Fillet*>(doc.addObject("PartDesign::Fillet", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"fillet-failed", "FreeCAD refused to create a PartDesign::Fillet"};
    }

    feature->Base.setValue(&base, matched.names);
    feature->UseAllEdges.setValue(matched.all_edges);
    feature->Radius.setValue(radius);
    params::apply(doc, *feature, "Radius", target.radius);
    App::DocumentObject* previous_tip = body.Tip.getValue();
    splice_dressup(body, base, *feature);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("fillet", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("base", json::Value::string(base.getNameInDocument()));
    out.set("radius", params::slot_json(*feature, "Radius", measure(feature->Radius.getValue())));
    out.set("edges_matched", json::Value::integer(static_cast<long long>(matched.names.size())));
    out.set("edges_length", json::Value::number(measure(matched.total_length)));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!body.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(body));
    out.set("shape", shape_of(body));
    return out;
}

json::Value Session::chamfer(const ChamferTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = body_for(doc, target.body);
    const std::vector<PartDesign::Feature*> based = features_for(doc, target.features, body);
    if (based.size() > 1) {
        throw Error{"too-many-features", "chamfer dresses one feature at a time, named " +
                                             std::to_string(based.size())};
    }
    PartDesign::Feature& base = *based.front();

    const double size = params::resolve(doc, target.size);
    if (size <= 0.0) {
        throw Error{"invalid-dimension", "size must be positive"};
    }
    const double angle = params::resolve(doc, target.angle);
    const bool angled = angle != 0.0;
    if (angled && !(angle > 0.0 && angle < 180.0)) {
        throw Error{"invalid-dimension", "angle must be between 0 and 180 degrees"};
    }

    const MatchedEdges matched = match_edges(base.Shape.getValue(), target.selection);

    const std::string wanted = target.name.empty() ? std::string("Chamfer") : target.name;
    auto* feature =
        static_cast<PartDesign::Chamfer*>(doc.addObject("PartDesign::Chamfer", wanted.c_str()));
    if (feature == nullptr) {
        throw Error{"chamfer-failed", "FreeCAD refused to create a PartDesign::Chamfer"};
    }

    feature->Base.setValue(&base, matched.names);
    feature->UseAllEdges.setValue(matched.all_edges);
    feature->ChamferType.setValue(angled ? "Distance and Angle" : "Equal distance");
    feature->Size.setValue(size);
    params::apply(doc, *feature, "Size", target.size);
    if (angled) {
        feature->Angle.setValue(angle);
        params::apply(doc, *feature, "Angle", target.angle);
    }
    App::DocumentObject* previous_tip = body.Tip.getValue();
    splice_dressup(body, base, *feature);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_or_rollback(doc, body, previous_tip, *feature);
    mark_changed(doc.getName());
    gui::fit_view();

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body.getNameInDocument()));
    out.set("chamfer", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("base", json::Value::string(base.getNameInDocument()));
    out.set("size", params::slot_json(*feature, "Size", measure(feature->Size.getValue())));
    if (angled) {
        out.set("angle", params::slot_json(*feature, "Angle", measure(feature->Angle.getValue())));
    }
    out.set("edges_matched", json::Value::integer(static_cast<long long>(matched.names.size())));
    out.set("edges_length", json::Value::number(measure(matched.total_length)));
    out.set("recompute", std::move(recomputed));
    out.set("solid", json::Value::boolean(!body.Shape.getValue().IsNull()));
    out.set("bounds", bounds_of(body));
    out.set("shape", shape_of(body));
    return out;
}

json::Value Session::set_slot(const SlotTarget& target)
{
    App::Document& doc = document_for(target.document);

    const bool on_sketch = target.kind == "sketch";
    const std::string slot =
        on_sketch ? sketch_slot_alias(target.slot)
                  : (target.slot == "Length" ? std::string("length") : target.slot);
    const bool on_revolve = target.kind == "revolve" || target.kind == "groove";
    Sketcher::SketchObject* sketch = nullptr;
    PartDesign::FeatureExtrude* extrude = nullptr;
    App::PropertyAngle* angle = nullptr;
    App::DocumentObject* object = nullptr;
    std::string path;

    if (on_sketch) {
        sketch = &sketch_for(doc, target.object);
        object = sketch;
        path = sketch_slot_path(slot);
    }
    else if (on_revolve) {
        if (slot != "angle") {
            throw Error{"unknown-slot", "a " + target.kind + " has only an angle slot"};
        }
        object = &revolve_for(doc, target.object, target.kind);
        angle = &angle_property(*object);
        path = "Angle";
    }
    else {
        if (slot != "length") {
            throw Error{"unknown-slot", "a " + target.kind + " has only a length slot"};
        }
        extrude = &extrude_for(doc, target.object, target.kind);
        object = extrude;
        path = "Length";
    }

    // Replacing a parameter with a literal is a real intention and a common
    // accident, and they are the same command. Only the accident is silent, so
    // the deliberate one has to say so.
    const params::Binding held = params::binding_of(*object, path);
    if (!held.expression.empty() && !target.value.bound() && !target.unbind) {
        throw Error{"slot-is-driven",
                    slot + " on " + object->getNameInDocument() + " follows " +
                        (held.parameter.empty() ? held.expression : held.parameter) +
                        ": drive that parameter instead, or pass --unbind to make this slot a "
                        "literal again"};
    }

    const double previous =
        on_sketch ? sketch_slot_value(*sketch, slot)
                  : (on_revolve ? angle->getValue() : extrude->Length.getValue());
    const double next = params::resolve(doc, target.value);
    if (on_revolve) {
        if (!(next > 0.0 && next <= 360.0)) {
            throw Error{"invalid-dimension", "angle must be between 0 and 360 degrees"};
        }
    }
    else if (!on_sketch && !(next > 0.0)) {
        throw Error{"invalid-dimension", "length must be positive"};
    }

    params::apply(doc, *object, path, target.value);
    if (on_sketch) {
        set_sketch_slot(*sketch, slot, next);
    }
    else if (on_revolve) {
        angle->setValue(next);
    }
    else {
        extrude->Length.setValue(next);
        extrude->Type.setValue("Length");
    }

    json::Value recomputed = recompute_document(doc);
    const bool failed = recomputed.find("failed")->as_bool() == true;
    if (!failed) {
        mark_changed(doc.getName());
    }

    const double landed =
        on_sketch ? sketch_slot_value(*sketch, slot)
                  : (on_revolve ? angle->getValue() : extrude->Length.getValue());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("object", json::Value::string(object->getNameInDocument()));
    out.set("kind", json::Value::string(target.kind));
    out.set("slot", json::Value::string(slot));
    out.set("value", params::slot_json(*object, path, measure(landed)));
    out.set("previous", json::Value::number(measure(previous)));
    out.set("recompute", std::move(recomputed));
    if (on_sketch) {
        sketch->solve(false);
        out.set("dof", json::Value::integer(sketch->getLastDoF()));
    }
    if (!failed) {
        PartDesign::Body* body = PartDesign::Body::findBodyOf(object);
        if (body != nullptr && !body->Shape.getValue().IsNull()) {
            out.set("bounds", bounds_of(*body));
            out.set("shape", shape_of(*body));
        }
    }
    return out;
}

json::Value Session::parameters(const std::string& document) const
{
    App::Document& doc = document_for(document);
    App::VarSet* registry = params::find(doc);

    json::Value list = json::Value::array();
    std::vector<std::string> culprits;
    std::vector<std::string> computed;
    std::vector<std::string> stalled;
    for (const std::string& name : params::names(doc)) {
        const params::Binding binding = params::binding_of(*registry, name);

        json::Value entry = json::Value::object();
        entry.set("name", json::Value::string(name));
        entry.set("value", json::Value::number(measure(params::value_of(doc, name))));
        entry.set("expression", binding.expression.empty()
                                    ? json::Value()
                                    : json::Value::string(binding.expression));

        const params::Evaluation checked = params::evaluate(*registry, name);
        entry.set("state", json::Value::string(params::state_name(checked.state)));
        if (!checked.error.empty()) {
            entry.set("error", json::Value::string(checked.error));
        }
        if (checked.state == params::State::Invalid) {
            culprits.push_back(name);
        }
        else if (!binding.expression.empty()) {
            computed.push_back(name);
            if (checked.state == params::State::NotEvaluated) {
                stalled.push_back(name);
            }
        }
        // Free: the expression engine already stores every reference, so the
        // dependency graph is read out of the document rather than tracked
        // alongside it and able to disagree with it.
        entry.set("drives", params::drives(doc, name));
        list.push(std::move(entry));
    }

    // Named dimensions nothing drives. Exactly what `param new` plus a rebind
    // would adopt, and all that a document written before the registry holds.
    json::Value orphans = json::Value::array();
    for (Sketcher::SketchObject* sketch : doc.getObjectsOfType<Sketcher::SketchObject>()) {
        for (const Sketcher::Constraint* constraint : sketch->Constraints.getValues()) {
            if (constraint->Name.empty() || !constraint->isDimensional()) {
                continue;
            }
            if (!params::binding_of(*sketch, "Constraints." + constraint->Name)
                     .expression.empty()) {
                continue;
            }
            json::Value entry = json::Value::object();
            entry.set("object", json::Value::string(sketch->getNameInDocument()));
            entry.set("slot", json::Value::string(constraint->Name));
            entry.set("type", json::Value::string(constraint->typeToString()));
            entry.set("value", json::Value::number(measure(constraint->getValue())));
            orphans.push(std::move(entry));
        }
    }

    // The registry's verdict on itself, computed here rather than in the client
    // so the table and the JSON cannot reach different conclusions about one
    // document. Once anything is invalid the whole VarSet stopped, so every
    // other computed row is unverified rather than fine: a row may read `ok`
    // because its arithmetic agrees with a sibling's stale number, which is
    // consistency with a value nobody stands behind. Literals are left out -
    // they compute nothing, so an aborted recompute cannot have skipped them.
    json::Value verdict = json::Value::object();
    verdict.set("trusted",
                json::Value::boolean(culprits.empty() && stalled.empty()));
    verdict.set("invalid", strings_json(culprits));
    verdict.set("unverified", strings_json(culprits.empty() ? stalled : computed));

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("parameters", std::move(list));
    out.set("registry", std::move(verdict));
    out.set("orphans", std::move(orphans));
    return out;
}

json::Value Session::declare_parameter(const ParamTarget& target)
{
    params::require_name(target.name);
    App::Document& doc = document_for(target.document);

    const bool known = params::exists(doc, target.name);
    if (target.must_be_new && known) {
        throw Error{"parameter-exists", target.name + " is already a parameter in " +
                                            doc.getName() + ": `param set` changes one"};
    }
    if (!target.must_be_new && !known) {
        for (Sketcher::SketchObject* sketch : doc.getObjectsOfType<Sketcher::SketchObject>()) {
            if (constraint_named(*sketch, target.name) < 0) {
                continue;
            }
            throw Error{"unknown-parameter",
                        target.name + " is a dimension on " + sketch->getNameInDocument() +
                            ", not a parameter: `param new " + target.name +
                            " <value>` then `sketch set " + target.name + " " + target.name +
                            " --sketch " + sketch->getNameInDocument() + "` adopts it"};
        }
        throw Error{"unknown-parameter", "no parameter named " + target.name + " in " +
                                             doc.getName() + ": `param new` declares one"};
    }

    // Whether this call is what brings the registry into existence, so a refusal
    // does not leave an empty one behind: every other refusal in this tool
    // leaves the document untouched, and a caller reading a nonzero exit as
    // "nothing happened" has to be right about that.
    const bool had_registry = params::find(doc) != nullptr;
    App::VarSet& registry = params::ensure(doc);
    const double previous = known ? params::value_of(doc, target.name) : 0.0;

    // Applied, then checked, then undone if the check refuses. Checking first
    // would be a different expression from the one that ends up bound - a cycle
    // is only a cycle once the binding is in place - so the only honest order
    // is to bind and be prepared to take it back.
    const params::Restore saved = params::capture(registry, target.name);
    try {
        // Inside the try, so a name the grammar rejects takes the registry with
        // it when this call is what created it, exactly as a bad expression does.
        params::require_usable_name(registry, target.name);

        if (target.expression.empty()) {
            params::declare(registry, target.name, target.value);
        }
        else {
            params::express(registry, target.name, target.expression);
        }

        // Only this row's own failure rolls back. A valid expression that lands
        // in a document some other row already poisoned reads NotEvaluated
        // here, which is also what a correct binding reads before the recompute
        // writes it, so neither is grounds to refuse.
        const params::Evaluation checked = params::evaluate(registry, target.name);
        if (checked.state == params::State::Invalid) {
            throw Error{"invalid-expression", checked.error};
        }
    }
    catch (...) {
        params::restore(registry, target.name, saved);
        if (!had_registry && params::names(doc).empty()) {
            doc.removeObject(registry.getNameInDocument());
        }
        (void)recompute_document(doc);
        throw;
    }

    json::Value recomputed = recompute_document(doc);
    const bool failed = recomputed.find("failed")->as_bool() == true;
    if (!failed) {
        mark_changed(doc.getName());
    }

    const params::Binding binding = params::binding_of(registry, target.name);

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("name", json::Value::string(target.name));
    out.set("value", json::Value::number(measure(params::value_of(doc, target.name))));
    out.set("expression", binding.expression.empty()
                              ? json::Value()
                              : json::Value::string(binding.expression));
    out.set("state", json::Value::string(params::state_name(
                         params::evaluate(registry, target.name).state)));
    out.set("created", json::Value::boolean(!known));
    out.set("previous", known ? json::Value::number(measure(previous)) : json::Value());
    out.set("drives", params::drives(doc, target.name));
    out.set("recompute", std::move(recomputed));
    return out;
}

double Session::evaluate_quantity(const std::string& document, const std::string& expression)
{
    App::Document& doc = document_for(document);

    // No caller can type this name, so nothing declared through `param new`
    // can ever collide with it.
    static constexpr const char* kScratch = "__quantity__";

    const bool had_registry = params::find(doc) != nullptr;
    App::VarSet& registry = params::ensure(doc);
    const params::Restore saved = params::capture(registry, kScratch);

    double value = 0.0;
    try {
        params::express(registry, kScratch, expression);
        const params::Evaluation checked = params::evaluate(registry, kScratch);
        if (checked.state == params::State::Invalid) {
            throw Error{"invalid-expression", checked.error};
        }
        (void)recompute_document(doc);
        value = params::value_of(doc, kScratch);
    }
    catch (...) {
        params::restore(registry, kScratch, saved);
        if (!had_registry && params::names(doc).empty()) {
            doc.removeObject(registry.getNameInDocument());
        }
        (void)recompute_document(doc);
        throw;
    }

    params::restore(registry, kScratch, saved);
    if (!had_registry && params::names(doc).empty()) {
        doc.removeObject(registry.getNameInDocument());
    }
    (void)recompute_document(doc);

    return measure(value);
}

json::Value Session::remove_parameter(const std::string& document,
                                      const std::string& name,
                                      bool force)
{
    App::Document& doc = document_for(document);
    if (!params::exists(doc, name)) {
        throw Error{"unknown-parameter",
                    "no parameter named " + name + " in " + doc.getName()};
    }

    const json::Value bound = params::drives(doc, name);
    const std::vector<json::Value>* slots = bound.as_array();
    const std::size_t count = slots == nullptr ? 0 : slots->size();
    if (count > 0 && !force) {
        throw Error{"parameter-in-use",
                    name + " drives " + std::to_string(count) +
                        " slots: `param list` names them, --force freezes each one at its "
                        "current value"};
    }

    // Clearing the expression leaves the number the parameter last computed, so
    // freeing a slot never moves the geometry by itself.
    json::Value froze = json::Value::array();
    if (slots != nullptr) {
        for (const json::Value& slot : *slots) {
            App::DocumentObject* object = doc.getObject(slot.find("object")->as_string()->c_str());
            if (object == nullptr) {
                continue;
            }
            const std::string path = *slot.find("slot")->as_string();
            params::apply(doc, *object, path, Slot{});
            froze.push(slot);
        }
    }

    params::find(doc)->removeDynamicProperty(name.c_str());

    json::Value recomputed = recompute_document(doc);
    if (recomputed.find("failed")->as_bool() == false) {
        mark_changed(doc.getName());
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("name", json::Value::string(name));
    out.set("froze", std::move(froze));
    out.set("recompute", std::move(recomputed));
    return out;
}

json::Value Session::remove_feature(const RemovalTarget& target)
{
    App::Document& doc = document_for(target.document);
    const RemovalPlan plan = plan_removal(doc, target.feature);

    if (!target.body.empty()) {
        PartDesign::Body& body = body_for(doc, target.body);
        if (plan.body != &body) {
            throw Error{"foreign-feature", std::string("feature ") + target.feature +
                                               " does not belong to " + body.getNameInDocument()};
        }
    }

    json::Value out = plan_json(plan);
    out.set("document", json::Value::string(doc.getName()));
    out.set("dry_run", json::Value::boolean(target.dry_run));
    if (target.dry_run) {
        return out;
    }

    apply_removal(doc, plan);

    json::Value recomputed = recompute_document(doc);
    if (recomputed.find("failed")->as_bool() == false) {
        mark_changed(doc.getName());
        gui::fit_view();
    }
    out.set("recompute", std::move(recomputed));
    return out;
}

json::Value Session::preview(const PreviewRequest& request)
{
    App::Document& doc = document_for(request.document);
    Part::Feature& target = preview_target(doc, request.object);

    const std::string path = request.path.empty()
                                 ? paths::preview_dir() + "/" + doc.getName() + ".stl"
                                 : request.path;
    paths::ensure_parent(path);

    MeshStats stats{};
    try {
        stats = write_binary_stl(target.Shape.getValue(), path, request.tessellation);
    }
    catch (const std::exception& error) {
        throw Error{"preview-failed", error.what()};
    }

    if (request.follow) {
        followed_[doc.getName()] =
            Followed{target.getNameInDocument(), path, request.tessellation};
    }
    else {
        followed_.erase(doc.getName());
    }
    dirty_.erase(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("object", json::Value::string(target.getNameInDocument()));
    out.set("path", json::Value::string(path));
    out.set("triangles", json::Value::integer(stats.triangles));
    out.set("bytes", json::Value::integer(stats.bytes));
    out.set("deflection", json::Value::number(request.tessellation.deflection));
    out.set("angular", json::Value::number(request.tessellation.angular));
    out.set("follow", json::Value::boolean(request.follow));
    return out;
}

json::Value Session::render(const RenderTarget& target)
{
    App::Document& doc = document_for(target.document);

    std::vector<Part::Feature*> drawn;
    if (target.object.empty()) {
        drawn = top_level_solids(doc);
        if (drawn.empty()) {
            throw Error{"unknown-shape",
                        std::string("document ") + doc.getName() + " has no shape"};
        }
    }
    else {
        drawn.push_back(&preview_target(doc, target.object));
    }

    std::vector<Facet> facets;
    Box box;
    for (Part::Feature* feature : drawn) {
        const TopoDS_Shape& shape = feature->Shape.getValue();
        if (shape.IsNull()) {
            continue;
        }
        try {
            const std::vector<Facet> part = tessellate(shape, target.tessellation);
            facets.insert(facets.end(), part.begin(), part.end());
        }
        catch (const std::exception& error) {
            throw Error{"render-failed", error.what()};
        }
        box.absorb(box_of(shape));
    }
    if (facets.empty()) {
        throw Error{"unknown-shape", "nothing in the document has a shape to draw"};
    }

    const std::string path = target.path.empty()
                                 ? paths::preview_dir() + "/" + doc.getName() + "-" +
                                       target.view + ".png"
                                 : target.path;
    paths::ensure_parent(path);

    RenderRequest request;
    request.view = target.view;
    request.width = target.width;
    request.height = target.height;

    RenderStats stats{};
    try {
        stats = write_png(facets, path, request);
    }
    catch (const std::exception& error) {
        throw Error{"render-failed", error.what()};
    }

    json::Value objects = json::Value::array();
    for (const Part::Feature* feature : drawn) {
        objects.push(json::Value::string(feature->getNameInDocument()));
    }

    json::Value camera = json::Value::object();
    camera.set("forward", vector3(stats.view.forward));
    camera.set("right", vector3(stats.view.right));
    camera.set("up", vector3(stats.view.up));

    json::Value axes = json::Value::object();
    axes.set("x", json::Value::string("red"));
    axes.set("y", json::Value::string("green"));
    axes.set("z", json::Value::string("blue"));

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("objects", std::move(objects));
    out.set("path", json::Value::string(path));
    out.set("view", json::Value::string(target.view));
    out.set("width", json::Value::integer(stats.width));
    out.set("height", json::Value::integer(stats.height));
    out.set("triangles", json::Value::integer(stats.triangles));
    out.set("bytes", json::Value::integer(stats.bytes));
    out.set("mm_per_pixel", json::Value::number(stats.mm_per_pixel));
    out.set("camera", std::move(camera));
    out.set("triad", std::move(axes));
    if (box.valid) {
        out.set("bbox", box_json(box));
    }
    return out;
}

json::Value Session::refresh_previews()
{
    json::Value out = json::Value::array();
    const std::set<std::string> changed = std::move(dirty_);
    dirty_.clear();

    for (const std::string& name : changed) {
        auto followed = followed_.find(name);
        if (followed == followed_.end()) {
            continue;
        }

        json::Value entry = json::Value::object();
        entry.set("document", json::Value::string(name));
        entry.set("object", json::Value::string(followed->second.object));
        entry.set("path", json::Value::string(followed->second.path));
        try {
            App::Document& doc = document_for(name);
            Part::Feature& target = preview_target(doc, followed->second.object);
            const MeshStats stats = write_binary_stl(target.Shape.getValue(),
                                                     followed->second.path,
                                                     followed->second.tessellation);
            entry.set("triangles", json::Value::integer(stats.triangles));
            entry.set("bytes", json::Value::integer(stats.bytes));
        }
        catch (const Error& error) {
            entry.set("error", json::Value::string(error.code + ": " + error.message));
        }
        catch (const std::exception& error) {
            entry.set("error", json::Value::string(error.what()));
        }
        out.push(std::move(entry));
    }
    return out;
}

void Session::mark_dirty(const std::string& document)
{
    if (followed_.find(document) != followed_.end()) {
        dirty_.insert(document);
    }
}

void Session::mark_changed(const std::string& document)
{
    mark_dirty(document);
    unsaved_.insert(document);
}

json::Value Session::recompute(const std::string& document)
{
    App::Document& doc = document_for(document);
    json::Value out = recompute_document(doc);
    if (const json::Value* failed = out.find("failed");
        failed != nullptr && failed->as_bool() == false) {
        mark_dirty(doc.getName());
    }
    return out;
}

json::Value Session::save(const std::string& document, const std::string& path)
{
    App::Document& doc = document_for(document);

    bool saved = false;
    if (path.empty()) {
        const char* file = doc.getFileName();
        if (file == nullptr || *file == '\0') {
            throw Error{"missing-path", "document was never saved, give a path"};
        }
        saved = doc.save();
    }
    else {
        saved = doc.saveAs(path.c_str());
    }
    if (!saved) {
        throw Error{"save-failed", std::string("FreeCAD could not save ") + doc.getName()};
    }
    unsaved_.erase(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("label", json::Value::string(doc.Label.getValue()));
    out.set("path", json::Value::string(doc.getFileName()));
    out.set("objects", json::Value::integer(static_cast<long long>(doc.getObjects().size())));
    return out;
}

json::Value Session::inspect(const std::string& document, bool features) const
{
    App::Document& doc = document_for(document);

    json::Value objects = json::Value::array();
    for (App::DocumentObject* object : doc.getObjects()) {
        if (is_scaffolding(*object)) {
            continue;
        }

        json::Value entry = json::Value::object();
        entry.set("name", json::Value::string(object->getNameInDocument()));
        entry.set("type", json::Value::string(object->getTypeId().getName()));
        entry.set("label", json::Value::string(object->Label.getValue()));
        entry.set("error", error_of(*object));
        if (auto* sketch = dynamic_cast<Sketcher::SketchObject*>(object)) {
            entry.set("sketch", sketch_detail(*sketch));
        }
        else if (auto* feature = dynamic_cast<Part::Feature*>(object)) {
            json::Value shape = shape_of(*feature);
            if (!shape.is_null()) {
                entry.set("shape", std::move(shape));
            }
        }
        objects.push(std::move(entry));
    }

    // The model as a reader sees it: one box, one volume, so a layout can be
    // checked numerically without opening anything.
    json::Value solids = json::Value::array();
    Box overall;
    for (Part::Feature* feature : top_level_solids(doc)) {
        json::Value entry = json::Value::object();
        entry.set("name", json::Value::string(feature->getNameInDocument()));
        entry.set("type", json::Value::string(feature->getTypeId().getName()));
        entry.set("shape", shape_of(*feature));
        solids.push(std::move(entry));
        overall.absorb(box_of(feature->Shape.getValue()));
    }

    json::Value out = document_summary(doc);
    out.set("objects", std::move(objects));
    out.set("solids", std::move(solids));
    if (overall.valid) {
        out.set("bbox", box_json(overall));
    }

    // Behind a flag rather than a verb: it is the same question `inspect`
    // already answers, asked about how the solid was built instead of what it
    // came out as.
    if (features) {
        json::Value bodies = json::Value::array();
        const std::map<App::DocumentObject*, std::string> consumed = consumed_bodies(doc);
        for (PartDesign::Body* body : doc.getObjectsOfType<PartDesign::Body>()) {
            json::Value tree = json::Value::array();
            double previous_volume = 0.0;
            Box previous_box;
            // `Group`, not `doc.getObjects()`: a dressup spliced after an
            // earlier feature sits before later ones in the body's own chain
            // even though it was created after them, and `volume_delta` below
            // is only meaningful walked in that chain order.
            for (App::DocumentObject* member : body->Group.getValues()) {
                if (dynamic_cast<PartDesign::Feature*>(member) == nullptr) {
                    continue;
                }
                json::Value entry = feature_of(*member);
                // What this feature alone contributed, not the body's running
                // total: the number a modeller needs, and the one hand-diffing
                // consecutive `inspect` replies used to stand in for.
                if (auto* solid = dynamic_cast<Part::Feature*>(member)) {
                    const TopoDS_Shape& shape = solid->Shape.getValue();
                    const double volume = volume_of(shape);
                    const Box box = box_of(shape);
                    entry.set("volume_delta",
                             json::Value::number(measure(volume - previous_volume)));
                    entry.set("bbox_delta", bbox_delta_json(previous_box, box));
                    previous_volume = volume;
                    previous_box = box;
                }
                tree.push(std::move(entry));
            }

            json::Value entry = json::Value::object();
            entry.set("body", json::Value::string(body->getNameInDocument()));
            entry.set("label", json::Value::string(body->Label.getValue()));
            entry.set("error", error_of(*body));
            auto found = consumed.find(body);
            if (found != consumed.end()) {
                entry.set("consumed_by", json::Value::string(found->second));
            }
            entry.set("features", std::move(tree));
            bodies.push(std::move(entry));
        }
        out.set("bodies", std::move(bodies));
    }
    return out;
}

}  // namespace ee
