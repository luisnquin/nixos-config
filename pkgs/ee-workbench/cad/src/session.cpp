#include "ee/session.hpp"

#include <algorithm>
#include <cmath>
#include <string>
#include <vector>

#include <App/Application.h>
#include <App/Datums.h>
#include <App/Document.h>
#include <App/DocumentObject.h>
#include <App/Origin.h>
#include <App/VarSet.h>
#include <Base/Interpreter.h>
#include <Base/Placement.h>
#include <Base/Rotation.h>
#include <Base/Vector3D.h>
#include <Mod/Part/App/Geometry.h>
#include <Mod/Part/App/Part2DObject.h>
#include <Mod/Part/App/PartFeature.h>
#include <Mod/PartDesign/App/Body.h>
#include <Mod/PartDesign/App/Feature.h>
#include <Mod/PartDesign/App/FeatureExtrude.h>
#include <Mod/PartDesign/App/FeaturePad.h>
#include <Mod/PartDesign/App/FeaturePocket.h>
#include <Mod/PartDesign/App/FeatureSketchBased.h>
#include <Mod/Sketcher/App/Constraint.h>
#include <Mod/Sketcher/App/GeoEnum.h>
#include <Mod/Sketcher/App/GeometryFacade.h>
#include <Mod/Sketcher/App/SketchObject.h>

#include <gp_Pnt.hxx>

#include <BRepBndLib.hxx>
#include <BRepGProp.hxx>
#include <Bnd_Box.hxx>
#include <GProp_GProps.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS_Shape.hxx>

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

/// Millimetres are reported to a micron. Anything finer is the arithmetic
/// talking, not the model: OCCT pads a bounding box by `Precision::Confusion()`
/// whatever gap you ask for, and an agent comparing numbers should not have to
/// tell -3 from -3.0000001.
double measure(double value)
{
    const double snapped = std::round(value * 1e6) / 1e6;
    return snapped == 0.0 ? 0.0 : snapped;
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

/// The shapes that make up the model as a reader sees it: every body, plus any
/// loose solid feature that no body owns. A pad inside a body is left out
/// because the body already carries its result.
std::vector<Part::Feature*> top_level_solids(App::Document& doc)
{
    std::vector<Part::Feature*> out;

    for (PartDesign::Body* body : doc.getObjectsOfType<PartDesign::Body>()) {
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

    const std::vector<PartDesign::Body*> bodies = doc.getObjectsOfType<PartDesign::Body>();
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

void require_empty(Sketcher::SketchObject& sketch)
{
    if (!sketch.getInternalGeometry().empty()) {
        throw Error{"sketch-not-empty",
                    std::string("sketch ") + sketch.getNameInDocument() +
                        " already has geometry"};
    }
}

/// Pins one sketch point to the origin with a named signed distance on each
/// axis. Always a pair, never the tidier coincidence a point already at the
/// origin would allow: an unnamed constraint cannot be bound to a parameter
/// afterwards, and a placement that can only be decided while drawing is the
/// same write-once defect one level up from the values it holds.
void pin_to_origin(Sketcher::SketchObject& sketch, int geo, PointPos pos, double x, double y)
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

    name_constraint(sketch, constrain(Sketcher::DistanceX, geo, pos, x), "x");
    name_constraint(sketch, constrain(Sketcher::DistanceY, geo, pos, y), "y");
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
    out.set("dimensions", dimensions_of(sketch));
    out.set("dof", json::Value::integer(sketch.getLastDoF()));
    out.set("fully_constrained", json::Value::boolean(sketch.getLastDoF() == 0));
    return out;
}

json::Value feature_of(App::DocumentObject& object)
{
    json::Value out = json::Value::object();
    out.set("name", json::Value::string(object.getNameInDocument()));
    out.set("type", json::Value::string(object.getTypeId().getName()));
    out.set("label", json::Value::string(object.Label.getValue()));
    out.set("error", error_of(object));

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

json::Value Session::new_sketch(const SketchTarget& target)
{
    App::Document& doc = document_for(target.document);
    PartDesign::Body& body = object_for<PartDesign::Body>(doc, target.body, "body");

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
    require_empty(sketch);

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
        constrain(Sketcher::Coincident, i, PointPos::end, (i + 1) % 4, PointPos::start, 0.0);
    }
    constrain(Sketcher::Horizontal, 0, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Horizontal, 2, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, 1, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, 3, PointPos::none, kGeoUndefined, PointPos::none, 0.0);

    if (target.centered) {
        // Pinning a corner would centre the rectangle only until someone drives
        // the width; a construction point the diagonal is symmetric about keeps
        // it centred through every later `param set`.
        Part::GeomPoint anchor(Base::Vector3d(x, y, 0.0));
        const int spot = sketch.addGeometry(&anchor, true);
        pin_to_origin(sketch, spot, PointPos::start, x, y);

        Sketcher::Constraint symmetric;
        symmetric.Type = Sketcher::Symmetric;
        symmetric.First = 0;
        symmetric.FirstPos = PointPos::start;
        symmetric.Second = 2;
        symmetric.SecondPos = PointPos::start;
        symmetric.Third = spot;
        symmetric.ThirdPos = PointPos::start;
        sketch.addConstraint(&symmetric);
    }
    else {
        pin_to_origin(sketch, 0, PointPos::start, left, bottom);
    }

    name_constraint(sketch,
                    constrain(Sketcher::DistanceX, 0, PointPos::start, 0, PointPos::end, width),
                    "width");
    name_constraint(sketch,
                    constrain(Sketcher::DistanceY, 1, PointPos::start, 1, PointPos::end, height),
                    "height");

    for (const auto& [slot, value] :
         {std::pair{"width", target.width}, std::pair{"height", target.height},
          std::pair{"x", target.x}, std::pair{"y", target.y}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("width", params::slot_json(sketch, "Constraints.width", measure(width)));
    out.set("height", params::slot_json(sketch, "Constraints.height", measure(height)));
    out.set("centered", json::Value::boolean(target.centered));
    out.set("corner", point(Base::Vector3d(left, bottom, 0.0)));
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
    require_empty(sketch);

    Part::GeomCircle geometry;
    geometry.setLocation(Base::Vector3d(x, y, 0.0));
    geometry.setRadius(radius);
    sketch.addGeometry(&geometry, false);

    Sketcher::Constraint dimension;
    dimension.Type = Sketcher::Radius;
    dimension.First = 0;
    dimension.FirstPos = PointPos::none;
    dimension.setValue(radius);
    name_constraint(sketch, sketch.addConstraint(&dimension), "radius");

    pin_to_origin(sketch, 0, PointPos::mid, x, y);

    for (const auto& [slot, value] : {std::pair{"radius", target.radius},
                                      std::pair{"x", target.x}, std::pair{"y", target.y}}) {
        params::apply(doc, sketch, sketch_slot_path(slot), value);
    }

    mark_changed(doc.getName());

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(sketch.getNameInDocument()));
    out.set("radius", params::slot_json(sketch, "Constraints.radius", measure(radius)));
    out.set("centre", point(Base::Vector3d(x, y, 0.0)));
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

ExtrudeParts resolve_extrude(const ExtrudeTarget& target)
{
    ExtrudeParts parts;
    parts.doc = &document_for(target.document);
    parts.body = &object_for<PartDesign::Body>(*parts.doc, target.body, "body");
    parts.profile =
        &sketch_for(*parts.doc, target.sketch, parts.body);

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
    // Midplane is deprecated in 1.1 and only forwards to SideType with a warning.
    feature->SideType.setValue(target.midplane ? "Symmetric" : "One side");
    feature->Reversed.setValue(target.reversed);
    parts.body->addObject(feature);

    // What the GUI does after a pad: the profile is consumed by the solid, so
    // leaving it visible only draws edges inside the part.
    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_document(doc);
    const bool failed = recomputed.find("failed")->as_bool() == true;
    if (!failed) {
        mark_changed(doc.getName());
        gui::fit_view();
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("pad", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("length",
            params::slot_json(*feature, "Length", measure(feature->Length.getValue())));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    if (!failed) {
        out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
        out.set("bounds", bounds_of(*parts.body));
        out.set("shape", shape_of(*parts.body));
    }
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
    feature->SideType.setValue(target.midplane ? "Symmetric" : "One side");
    feature->Reversed.setValue(target.reversed);
    parts.body->addObject(feature);

    parts.profile->Visibility.setValue(false);
    feature->Visibility.setValue(true);

    json::Value recomputed = recompute_document(doc);
    const bool failed = recomputed.find("failed")->as_bool() == true;
    if (!failed) {
        mark_changed(doc.getName());
        gui::fit_view();
    }

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(parts.body->getNameInDocument()));
    out.set("sketch", json::Value::string(parts.profile->getNameInDocument()));
    out.set("pocket", json::Value::string(feature->getNameInDocument()));
    out.set("label", json::Value::string(feature->Label.getValue()));
    out.set("length",
            params::slot_json(*feature, "Length", measure(feature->Length.getValue())));
    out.set("through_all", json::Value::boolean(target.through_all));
    out.set("midplane", json::Value::boolean(target.midplane));
    out.set("reversed", json::Value::boolean(target.reversed));
    out.set("recompute", std::move(recomputed));
    if (!failed) {
        out.set("solid", json::Value::boolean(!parts.body->Shape.getValue().IsNull()));
        out.set("bounds", bounds_of(*parts.body));
        out.set("shape", shape_of(*parts.body));
    }
    return out;
}

json::Value Session::set_slot(const SlotTarget& target)
{
    App::Document& doc = document_for(target.document);

    const bool on_sketch = target.kind == "sketch";
    const std::string slot =
        on_sketch ? sketch_slot_alias(target.slot)
                  : (target.slot == "Length" ? std::string("length") : target.slot);
    Sketcher::SketchObject* sketch = nullptr;
    PartDesign::FeatureExtrude* extrude = nullptr;
    App::DocumentObject* object = nullptr;
    std::string path;

    if (on_sketch) {
        sketch = &sketch_for(doc, target.object);
        object = sketch;
        path = sketch_slot_path(slot);
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
        on_sketch ? sketch_slot_value(*sketch, slot) : extrude->Length.getValue();
    const double next = params::resolve(doc, target.value);
    if (!on_sketch && !(next > 0.0)) {
        throw Error{"invalid-dimension", "length must be positive"};
    }

    params::apply(doc, *object, path, target.value);
    if (on_sketch) {
        set_sketch_slot(*sketch, slot, next);
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
        on_sketch ? sketch_slot_value(*sketch, slot) : extrude->Length.getValue();

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

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("parameters", std::move(list));
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
    return document_summary(doc);
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
        for (PartDesign::Body* body : doc.getObjectsOfType<PartDesign::Body>()) {
            json::Value tree = json::Value::array();
            for (App::DocumentObject* member : doc.getObjects()) {
                if (dynamic_cast<PartDesign::Feature*>(member) == nullptr ||
                    PartDesign::Body::findBodyOf(member) != body) {
                    continue;
                }
                tree.push(feature_of(*member));
            }

            json::Value entry = json::Value::object();
            entry.set("body", json::Value::string(body->getNameInDocument()));
            entry.set("label", json::Value::string(body->Label.getValue()));
            entry.set("error", error_of(*body));
            entry.set("features", std::move(tree));
            bodies.push(std::move(entry));
        }
        out.set("bodies", std::move(bodies));
    }
    return out;
}

}  // namespace ee
