#include "ee/session.hpp"

#include <algorithm>
#include <string>
#include <vector>

#include <App/Application.h>
#include <App/Datums.h>
#include <App/Document.h>
#include <App/DocumentObject.h>
#include <App/Origin.h>
#include <Base/Interpreter.h>
#include <Base/Vector3D.h>
#include <Mod/Part/App/Geometry.h>
#include <Mod/PartDesign/App/Body.h>
#include <Mod/Sketcher/App/Constraint.h>
#include <Mod/Sketcher/App/GeoEnum.h>
#include <Mod/Sketcher/App/SketchObject.h>

namespace ee {
namespace {

using Sketcher::PointPos;

/// Sketcher marks "no second element" with this sentinel rather than an
/// optional, and repeats it in the saved file.
constexpr int kGeoUndefined = Sketcher::GeoEnum::GeoUndef;
/// The sketch origin point lives on an implicit geometry with index -1.
constexpr int kRootPoint = Sketcher::GeoEnum::RtPnt;

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
    out.set("x", json::Value::number(value.x));
    out.set("y", json::Value::number(value.y));
    return out;
}

json::Value geometry_of(const Sketcher::SketchObject& sketch)
{
    json::Value out = json::Value::array();
    int index = 0;
    for (const Part::Geometry* geo : sketch.getInternalGeometry()) {
        json::Value entry = json::Value::object();
        entry.set("index", json::Value::integer(index++));
        entry.set("type", json::Value::string(geo->getTypeId().getName()));
        if (const auto* line = dynamic_cast<const Part::GeomLineSegment*>(geo)) {
            entry.set("start", point(line->getStartPoint()));
            entry.set("end", point(line->getEndPoint()));
            entry.set("length", json::Value::number(
                                    (line->getEndPoint() - line->getStartPoint()).Length()));
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
        if (constraint->isDimensional()) {
            entry.set("value", json::Value::number(constraint->getValue()));
        }
        entry.set("driving", json::Value::boolean(constraint->isDriving));
        if (!constraint->Name.empty()) {
            entry.set("name", json::Value::string(constraint->Name));
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
    out.set("geometry", geometry_of(sketch));
    out.set("constraints", constraints_of(sketch));
    out.set("dof", json::Value::integer(sketch.getLastDoF()));
    out.set("fully_constrained", json::Value::boolean(sketch.getLastDoF() == 0));
    out.set("redundant", json::Value::boolean(sketch.getLastHasRedundancies()));
    return out;
}

json::Value document_summary(const App::Document& doc)
{
    json::Value out = json::Value::object();
    out.set("name", json::Value::string(doc.getName()));
    out.set("label", json::Value::string(doc.Label.getValue()));
    const char* file = doc.getFileName();
    out.set("file", (file != nullptr && *file != '\0') ? json::Value::string(file) : json::Value());
    out.set("objects", json::Value::integer(static_cast<long long>(doc.getObjects().size())));
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

    json::Value out = json::Value::object();
    out.set("freecad", std::move(freecad));
    out.set("active", active != nullptr ? json::Value::string(active->getName()) : json::Value());
    out.set("documents", std::move(documents));
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

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(body->getNameInDocument()));
    out.set("label", json::Value::string(body->Label.getValue()));
    return out;
}

json::Value Session::new_sketch(const std::string& document,
                                const std::string& body,
                                const std::string& plane,
                                const std::string& name)
{
    App::Document& doc = document_for(document);
    PartDesign::Body& target = object_for<PartDesign::Body>(doc, body, "body");

    App::Origin* origin = target.getOrigin();
    if (origin == nullptr) {
        throw Error{"no-origin", std::string("body ") + target.getNameInDocument() +
                                     " has no origin planes"};
    }

    const std::string wanted_plane = plane.empty() ? std::string("xy") : plane;
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

    const std::string wanted = name.empty() ? std::string("Sketch") : name;
    auto* sketch = static_cast<Sketcher::SketchObject*>(
        doc.addObject("Sketcher::SketchObject", wanted.c_str()));
    if (sketch == nullptr) {
        throw Error{"sketch-failed", "FreeCAD refused to create a Sketcher::SketchObject"};
    }

    sketch->AttachmentSupport.setValue(datum, "");
    sketch->MapMode.setValue("FlatFace");
    target.addObject(sketch);

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("body", json::Value::string(target.getNameInDocument()));
    out.set("sketch", json::Value::string(sketch->getNameInDocument()));
    out.set("label", json::Value::string(sketch->Label.getValue()));
    out.set("plane", json::Value::string(datum->getNameInDocument()));
    return out;
}

json::Value Session::rectangle(const std::string& document,
                               const std::string& sketch,
                               double width,
                               double height)
{
    if (!(width > 0.0) || !(height > 0.0)) {
        throw Error{"invalid-dimension", "width and height must be positive"};
    }

    App::Document& doc = document_for(document);
    Sketcher::SketchObject& target = object_for<Sketcher::SketchObject>(doc, sketch, "sketch");
    if (!target.getInternalGeometry().empty()) {
        throw Error{"sketch-not-empty",
                    std::string("sketch ") + target.getNameInDocument() + " already has geometry"};
    }

    const Base::Vector3d corners[4] = {Base::Vector3d(0.0, 0.0, 0.0),
                                       Base::Vector3d(width, 0.0, 0.0),
                                       Base::Vector3d(width, height, 0.0),
                                       Base::Vector3d(0.0, height, 0.0)};
    for (int i = 0; i < 4; ++i) {
        Part::GeomLineSegment line;
        line.setPoints(corners[i], corners[(i + 1) % 4]);
        target.addGeometry(&line, false);
    }

    auto constrain = [&target](Sketcher::ConstraintType type,
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
        target.addConstraint(&constraint);
    };

    for (int i = 0; i < 4; ++i) {
        constrain(Sketcher::Coincident, i, PointPos::end, (i + 1) % 4, PointPos::start, 0.0);
    }
    constrain(Sketcher::Horizontal, 0, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Horizontal, 2, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, 1, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Vertical, 3, PointPos::none, kGeoUndefined, PointPos::none, 0.0);
    constrain(Sketcher::Coincident, 0, PointPos::start, kRootPoint, PointPos::start, 0.0);
    constrain(Sketcher::DistanceX, 0, PointPos::start, 0, PointPos::end, width);
    constrain(Sketcher::DistanceY, 1, PointPos::start, 1, PointPos::end, height);

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("sketch", json::Value::string(target.getNameInDocument()));
    out.set("width", json::Value::number(width));
    out.set("height", json::Value::number(height));
    const json::Value detail = sketch_detail(target);
    for (const char* field : {"geometry", "constraints", "dof", "fully_constrained", "redundant"}) {
        const json::Value* value = detail.find(field);
        if (value != nullptr) {
            out.set(field, *value);
        }
    }
    return out;
}

json::Value Session::recompute(const std::string& document)
{
    App::Document& doc = document_for(document);
    bool failed = false;
    const int touched = doc.recompute({}, false, &failed);

    json::Value out = json::Value::object();
    out.set("document", json::Value::string(doc.getName()));
    out.set("recomputed", json::Value::integer(touched));
    out.set("failed", json::Value::boolean(failed));
    if (failed) {
        json::Value errors = json::Value::array();
        for (const App::DocumentObject* object : doc.getObjects()) {
            if (!object->isError()) {
                continue;
            }
            json::Value entry = json::Value::object();
            entry.set("object", json::Value::string(object->getNameInDocument()));
            entry.set("status", json::Value::string(object->getStatusString()));
            errors.push(std::move(entry));
        }
        out.set("errors", std::move(errors));
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
    return document_summary(doc);
}

json::Value Session::inspect(const std::string& document) const
{
    App::Document& doc = document_for(document);

    json::Value objects = json::Value::array();
    for (App::DocumentObject* object : doc.getObjects()) {
        json::Value entry = json::Value::object();
        entry.set("name", json::Value::string(object->getNameInDocument()));
        entry.set("type", json::Value::string(object->getTypeId().getName()));
        entry.set("label", json::Value::string(object->Label.getValue()));
        if (auto* sketch = dynamic_cast<Sketcher::SketchObject*>(object)) {
            entry.set("sketch", sketch_detail(*sketch));
        }
        objects.push(std::move(entry));
    }

    json::Value out = document_summary(doc);
    out.set("objects", std::move(objects));
    return out;
}

}  // namespace ee
