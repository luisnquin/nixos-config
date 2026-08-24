#include "ee/protocol.hpp"

#include <optional>
#include <string>
#include <utility>

#include <Base/Exception.h>
#include <Base/Interpreter.h>

namespace ee {
namespace {

const json::Value* field(const json::Value* params, const char* name)
{
    return params == nullptr ? nullptr : params->find(name);
}

std::string string_param(const json::Value* params, const char* name)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return {};
    }
    const std::string* text = value->as_string();
    if (text == nullptr) {
        throw Error{"invalid-param", std::string(name) + " must be a string"};
    }
    return *text;
}

std::vector<std::string> string_array_param(const json::Value* params, const char* name)
{
    std::vector<std::string> out;
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return out;
    }
    const std::vector<json::Value>* items = value->as_array();
    if (items == nullptr) {
        throw Error{"invalid-param", std::string(name) + " must be an array of strings"};
    }
    out.reserve(items->size());
    for (const json::Value& item : *items) {
        const std::string* text = item.as_string();
        if (text == nullptr) {
            throw Error{"invalid-param", std::string(name) + " must be an array of strings"};
        }
        out.push_back(*text);
    }
    return out;
}

/// Absolute only. This process is a daemon whose working directory is whichever
/// one the `ee` that happened to start it was run from, and it outlives that
/// shell; a relative path here would write a file somewhere the caller never
/// named. The client resolves before sending, so anything relative reaching
/// this point is a caller that skipped it.
std::string path_param(const json::Value* params, const char* name)
{
    std::string path = string_param(params, name);
    if (!path.empty() && path.front() != '/') {
        throw Error{"invalid-param",
                    std::string(name) + " must be absolute, got " + path};
    }
    return path;
}

double required_number(const json::Value* params, const char* name)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        throw Error{"missing-param", std::string(name) + " is required"};
    }
    const std::optional<double> number = value->as_double();
    if (!number.has_value()) {
        throw Error{"invalid-param", std::string(name) + " must be a number"};
    }
    return *number;
}

double number_param(const json::Value* params, const char* name, double fallback)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return fallback;
    }
    return required_number(params, name);
}

long long integer_param(const json::Value* params, const char* name, long long fallback)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return fallback;
    }
    const std::optional<long long> number = value->as_integer();
    if (!number.has_value()) {
        throw Error{"invalid-param", std::string(name) + " must be an integer"};
    }
    return *number;
}

bool bool_param(const json::Value* params, const char* name, bool fallback)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return fallback;
    }
    const std::optional<bool> flag = value->as_bool();
    if (!flag.has_value()) {
        throw Error{"invalid-param", std::string(name) + " must be a boolean"};
    }
    return *flag;
}

/// A numeric slot takes a number, the name of a parameter, or `{"expression":
/// "..."}`, and the wire keeps them apart by JSON shape rather than by a
/// sigil. Every such field accepts all three on every call: which form a slot
/// holds is a decision the caller can revisit, not one the object fossilizes
/// when it is created. An expression is evaluated once, through the same
/// grammar `param new` binds a name to, and only its result is kept - a slot
/// never carries an expression of its own.
Slot slot_param(Session& session, const json::Value* params, const char* name, double fallback)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        return Slot{fallback, {}};
    }
    if (const std::string* text = value->as_string()) {
        ::ee::params::require_name(*text);
        return Slot{0.0, *text};
    }
    if (value->kind() == json::Value::Kind::Object) {
        const json::Value* expression = field(value, "expression");
        const std::string* text = expression != nullptr ? expression->as_string() : nullptr;
        if (text == nullptr) {
            throw Error{"invalid-param", std::string(name) + ".expression must be a string"};
        }
        return Slot{session.evaluate_quantity(string_param(params, "document"), *text), {}};
    }
    return Slot{required_number(params, name), {}};
}

Slot required_slot(Session& session, const json::Value* params, const char* name)
{
    const json::Value* value = field(params, name);
    if (value == nullptr || value->is_null()) {
        throw Error{"missing-param", std::string(name) + " is required"};
    }
    return slot_param(session, params, name, 0.0);
}

/// Fillet/chamfer's edge predicates, composed by AND on the session side. Only
/// `longer_than`/`shorter_than` need a presence flag here - the axis fields
/// already have empty-string as their own "unset".
EdgeSelection edge_selection_param(const json::Value* params)
{
    EdgeSelection out;
    out.parallel = string_param(params, "parallel");
    out.near_min = string_param(params, "near_min");
    out.near_max = string_param(params, "near_max");
    if (field(params, "longer_than") != nullptr) {
        out.has_longer_than = true;
        out.longer_than = required_number(params, "longer_than");
    }
    if (field(params, "shorter_than") != nullptr) {
        out.has_shorter_than = true;
        out.shorter_than = required_number(params, "shorter_than");
    }
    return out;
}

json::Value envelope(bool ok, const json::Value& id)
{
    json::Value out = json::Value::object();
    out.set("ok", json::Value::boolean(ok));
    out.set("protocol", json::Value::integer(kProtocol));
    // On every reply, not only `session.status`: a client that never asks for
    // status still has to find out it is talking to a session left behind by an
    // older generation, and it costs nothing to tell it on the round trip it
    // was already making.
    out.set("build", json::Value::string(build_id()));
    out.set("id", id);
    return out;
}

std::string failure(const json::Value& id, const std::string& code, const std::string& message)
{
    json::Value error = json::Value::object();
    error.set("code", json::Value::string(code));
    error.set("message", json::Value::string(message));

    json::Value out = envelope(false, id);
    out.set("error", std::move(error));
    return out.dump();
}

}  // namespace

const char* build_id()
{
#ifdef EE_BUILD_ID
    return EE_BUILD_ID;
#else
    return "";
#endif
}

std::string protocol_failure(const std::string& code, const std::string& message)
{
    return failure(json::Value(), code, message);
}

bool Protocol::handle_line(const std::string& line, std::string& reply)
{
    if (line.find_first_not_of(" \t\r") == std::string::npos) {
        reply = failure(json::Value(), "malformed", "empty request");
        return true;
    }

    std::string parse_error;
    const std::optional<json::Value> request = json::parse(line, parse_error);
    if (!request.has_value()) {
        reply = failure(json::Value(), "malformed", parse_error);
        return true;
    }
    if (request->kind() != json::Value::Kind::Object) {
        reply = failure(json::Value(), "malformed", "request must be an object");
        return true;
    }

    const json::Value* id_field = request->find("id");
    const json::Value id = id_field != nullptr ? *id_field : json::Value();

    const json::Value* method_field = request->find("method");
    const std::string* method = method_field != nullptr ? method_field->as_string() : nullptr;
    if (method == nullptr) {
        reply = failure(id, "malformed", "method is required");
        return true;
    }

    if (const json::Value* protocol = request->find("protocol")) {
        const std::optional<long long> version = protocol->as_integer();
        if (!version.has_value()) {
            reply = failure(id, "protocol-mismatch", "protocol must be an integer");
            return true;
        }
        // A protocol-mismatched session is the one a caller most needs to get
        // out of: refusing session.status/document.save/server.shutdown here
        // too would leave `session stop` unable to reach a server old or new
        // enough to misread the request it is carrying. These three do not
        // depend on the wire shape that changed, so answering anyway is safe.
        const bool rescuable = *method == "session.status" || *method == "document.save"
            || *method == "server.shutdown";
        if (*version != kProtocol && !rescuable) {
            reply = failure(id, "protocol-mismatch",
                            "server speaks protocol " + std::to_string(kProtocol) +
                                ", client sent " + std::to_string(*version));
            return true;
        }
    }

    if (*method == "server.shutdown") {
        stopping_ = true;
        json::Value out = envelope(true, id);
        out.set("result", json::Value::object());
        reply = out.dump();
        return false;
    }

    try {
        Base::PyGILStateLocker lock;
        json::Value result = dispatch(*method, request->find("params"));
        json::Value out = envelope(true, id);
        out.set("result", std::move(result));
        reply = out.dump();
    }
    catch (const Error& error) {
        reply = failure(id, error.code, error.message);
        return true;
    }
    catch (const Base::Exception& error) {
        reply = failure(id, "freecad", error.what());
        return true;
    }
    catch (const std::exception& error) {
        reply = failure(id, "internal", error.what());
        return true;
    }

    // Only a request that succeeded can have left a mesh behind.
    if (session_.preview_pending() && on_stale_) {
        on_stale_();
    }
    return true;
}

bool Protocol::has_unsaved() const
{
    Base::PyGILStateLocker lock;
    return !session_.unsaved_documents().empty();
}

void Protocol::refresh_previews()
{
    Base::PyGILStateLocker lock;
    session_.refresh_previews();
}

json::Value Protocol::dispatch(const std::string& method, const json::Value* params)
{
    if (params != nullptr && params->kind() != json::Value::Kind::Object) {
        throw Error{"invalid-param", "params must be an object"};
    }

    if (method == "session.status") {
        return session_.status();
    }
    if (method == "document.new") {
        return session_.new_document(string_param(params, "name"));
    }
    if (method == "document.open") {
        return session_.open_document(path_param(params, "path"));
    }
    if (method == "document.recompute") {
        return session_.recompute(string_param(params, "document"));
    }
    if (method == "document.save") {
        return session_.save(string_param(params, "document"), path_param(params, "path"));
    }
    if (method == "document.inspect") {
        // `tree` is the same question asked by a different name: both put the
        // build order behind the reply, so either flag turns it on.
        return session_.inspect(string_param(params, "document"),
                                bool_param(params, "features", false) ||
                                    bool_param(params, "tree", false));
    }
    if (method == "body.new") {
        return session_.new_body(string_param(params, "document"), string_param(params, "name"));
    }
    if (method == "body.union" || method == "body.cut" || method == "body.intersect") {
        BooleanTarget target;
        target.document = string_param(params, "document");
        target.base = string_param(params, "base");
        target.tool = string_array_param(params, "tool");
        target.name = string_param(params, "name");
        target.operation = method == "body.union"   ? "union"
                           : method == "body.cut"     ? "cut"
                                                       : "intersect";
        return session_.body_boolean(target);
    }
    if (method == "sketch.new") {
        SketchTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.plane = string_param(params, "plane");
        target.name = string_param(params, "name");
        target.offset_x = slot_param(session_, params, "offset_x", 0.0);
        target.offset_y = slot_param(session_, params, "offset_y", 0.0);
        target.offset_z = slot_param(session_, params, "offset_z", 0.0);
        target.rotate = slot_param(session_, params, "rotate", 0.0);
        return session_.new_sketch(target);
    }
    if (method == "sketch.rectangle") {
        RectangleTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.width = required_slot(session_, params, "width");
        target.height = required_slot(session_, params, "height");
        target.x = slot_param(session_, params, "x", 0.0);
        target.y = slot_param(session_, params, "y", 0.0);
        target.centered = bool_param(params, "centered", false);
        return session_.rectangle(target);
    }
    if (method == "sketch.circle") {
        CircleTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.radius = required_slot(session_, params, "radius");
        target.x = slot_param(session_, params, "x", 0.0);
        target.y = slot_param(session_, params, "y", 0.0);
        return session_.circle(target);
    }
    if (method == "sketch.line") {
        LineTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.x1 = required_slot(session_, params, "x1");
        target.y1 = required_slot(session_, params, "y1");
        target.x2 = required_slot(session_, params, "x2");
        target.y2 = required_slot(session_, params, "y2");
        return session_.line(target);
    }
    if (method == "sketch.arc") {
        ArcTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.x1 = required_slot(session_, params, "x1");
        target.y1 = required_slot(session_, params, "y1");
        target.x2 = required_slot(session_, params, "x2");
        target.y2 = required_slot(session_, params, "y2");
        target.radius = required_slot(session_, params, "radius");
        target.large = bool_param(params, "large", false);
        return session_.arc(target);
    }
    if (method == "sketch.polyline") {
        PolylineTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.close = bool_param(params, "close", false);
        const json::Value* points = field(params, "points");
        const std::vector<json::Value>* items = points != nullptr ? points->as_array() : nullptr;
        if (items == nullptr) {
            throw Error{"missing-param", "points must be an array of [x, y] pairs"};
        }
        for (std::size_t i = 0; i < items->size(); ++i) {
            const std::vector<json::Value>* pair = (*items)[i].as_array();
            if (pair == nullptr || pair->size() != 2) {
                throw Error{"invalid-param",
                            "point " + std::to_string(i + 1) + " must be an [x, y] pair"};
            }
            auto coordinate = [&](std::size_t axis) {
                const json::Value& value = (*pair)[axis];
                if (const std::string* text = value.as_string()) {
                    ::ee::params::require_name(*text);
                    return Slot{0.0, *text};
                }
                const std::optional<double> number = value.as_double();
                if (!number.has_value()) {
                    throw Error{"invalid-param", "point " + std::to_string(i + 1) +
                                                     " must hold numbers or parameter names"};
                }
                return Slot{*number, {}};
            };
            target.points.emplace_back(coordinate(0), coordinate(1));
        }
        return session_.polyline(target);
    }
    if (method == "pad.new" || method == "pocket.new") {
        const bool cutting = method == "pocket.new";
        ExtrudeTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.sketch = string_param(params, "sketch");
        target.name = string_param(params, "name");
        target.through_all = cutting && bool_param(params, "through_all", false);
        target.length = target.through_all ? slot_param(session_, params, "length", 0.0)
                                           : required_slot(session_, params, "length");
        target.midplane = bool_param(params, "midplane", false);
        target.reversed = bool_param(params, "reversed", false);
        target.taper = slot_param(session_, params, "taper", 0.0);
        return cutting ? session_.pocket(target) : session_.pad(target);
    }
    if (method == "revolve.new" || method == "groove.new") {
        const bool cutting = method == "groove.new";
        RevolveTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.sketch = string_param(params, "sketch");
        target.name = string_param(params, "name");
        target.axis = string_param(params, "axis");
        target.angle = required_slot(session_, params, "angle");
        target.midplane = bool_param(params, "midplane", false);
        target.reversed = bool_param(params, "reversed", false);
        return cutting ? session_.groove(target) : session_.revolve(target);
    }
    if (method == "loft.new" || method == "loft.pocket") {
        LoftTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.sketches = string_array_param(params, "sketch");
        target.ruled = bool_param(params, "ruled", false);
        target.closed = bool_param(params, "closed", false);
        target.name = string_param(params, "name");
        return method == "loft.new" ? session_.loft_new(target) : session_.loft_pocket(target);
    }
    if (method == "mirror.new") {
        MirrorTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.plane = string_param(params, "plane");
        target.features = string_array_param(params, "feature");
        target.name = string_param(params, "name");
        return session_.mirror(target);
    }
    if (method == "pattern.linear.new") {
        LinearPatternTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.direction = string_param(params, "direction");
        target.features = string_array_param(params, "feature");
        target.name = string_param(params, "name");
        target.spacing = required_slot(session_, params, "spacing");
        target.count = static_cast<int>(integer_param(params, "count", 2));
        target.reversed = bool_param(params, "reversed", false);
        return session_.pattern_linear(target);
    }
    if (method == "pattern.polar.new") {
        PolarPatternTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.axis = string_param(params, "axis");
        target.features = string_array_param(params, "feature");
        target.name = string_param(params, "name");
        target.angle = required_slot(session_, params, "angle");
        target.count = static_cast<int>(integer_param(params, "count", 2));
        return session_.pattern_polar(target);
    }
    if (method == "fillet.new") {
        FilletTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.features = string_array_param(params, "feature");
        target.name = string_param(params, "name");
        target.radius = required_slot(session_, params, "radius");
        target.selection = edge_selection_param(params);
        return session_.fillet(target);
    }
    if (method == "chamfer.new") {
        ChamferTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.features = string_array_param(params, "feature");
        target.name = string_param(params, "name");
        target.size = required_slot(session_, params, "size");
        target.angle = slot_param(session_, params, "angle", 0.0);
        target.selection = edge_selection_param(params);
        return session_.chamfer(target);
    }
    // One method for every numeric slot in the model. `kind` says how to
    // resolve an unnamed object; `slot` is the dimension's own name, which is
    // why every dimension gets one whether the caller asked or not.
    if (method == "slot.set") {
        SlotTarget target;
        target.document = string_param(params, "document");
        target.object = string_param(params, "object");
        target.kind = string_param(params, "kind");
        target.slot = string_param(params, "slot");
        target.value = required_slot(session_, params, "value");
        target.unbind = bool_param(params, "unbind", false);
        return session_.set_slot(target);
    }
    if (method == "param.new" || method == "param.set") {
        ParamTarget target;
        target.document = string_param(params, "document");
        target.name = string_param(params, "name");
        target.expression = string_param(params, "expression");
        if (target.expression.empty()) {
            target.value = required_number(params, "value");
        }
        target.must_be_new = method == "param.new";
        return session_.declare_parameter(target);
    }
    if (method == "feature.remove") {
        RemovalTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.feature = string_param(params, "feature");
        target.dry_run = bool_param(params, "dry_run", false);
        return session_.remove_feature(target);
    }
    if (method == "param.list") {
        return session_.parameters(string_param(params, "document"));
    }
    if (method == "param.remove") {
        return session_.remove_parameter(string_param(params, "document"),
                                         string_param(params, "name"),
                                         bool_param(params, "force", false));
    }
    if (method == "preview.export") {
        PreviewRequest request;
        request.document = string_param(params, "document");
        request.object = string_param(params, "object");
        request.path = path_param(params, "path");
        request.tessellation.deflection =
            number_param(params, "deflection", request.tessellation.deflection);
        request.tessellation.angular =
            number_param(params, "angular", request.tessellation.angular);
        request.follow = bool_param(params, "follow", true);
        return session_.preview(request);
    }
    if (method == "preview.render") {
        RenderTarget target;
        target.document = string_param(params, "document");
        target.object = string_param(params, "object");
        target.path = path_param(params, "path");
        const std::string view = string_param(params, "view");
        if (!view.empty()) {
            target.view = view;
        }
        target.width = static_cast<int>(integer_param(params, "width", target.width));
        target.height = static_cast<int>(integer_param(params, "height", target.height));
        target.tessellation.deflection =
            number_param(params, "deflection", target.tessellation.deflection);
        target.tessellation.angular =
            number_param(params, "angular", target.tessellation.angular);
        return session_.render(target);
    }
    throw Error{"unknown-method", "no method named " + method};
}

}  // namespace ee
