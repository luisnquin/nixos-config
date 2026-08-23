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

json::Value envelope(bool ok, const json::Value& id)
{
    json::Value out = json::Value::object();
    out.set("ok", json::Value::boolean(ok));
    out.set("protocol", json::Value::integer(kProtocol));
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

    if (const json::Value* protocol = request->find("protocol")) {
        const std::optional<long long> version = protocol->as_integer();
        if (!version.has_value()) {
            reply = failure(id, "protocol-mismatch", "protocol must be an integer");
            return true;
        }
        if (*version != kProtocol) {
            reply = failure(id, "protocol-mismatch",
                            "server speaks protocol " + std::to_string(kProtocol) +
                                ", client sent " + std::to_string(*version));
            return true;
        }
    }

    const json::Value* method_field = request->find("method");
    const std::string* method = method_field != nullptr ? method_field->as_string() : nullptr;
    if (method == nullptr) {
        reply = failure(id, "malformed", "method is required");
        return true;
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
        return session_.inspect(string_param(params, "document"));
    }
    if (method == "body.new") {
        return session_.new_body(string_param(params, "document"), string_param(params, "name"));
    }
    if (method == "sketch.new") {
        SketchTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.plane = string_param(params, "plane");
        target.name = string_param(params, "name");
        target.offset_x = number_param(params, "offset_x", 0.0);
        target.offset_y = number_param(params, "offset_y", 0.0);
        target.offset_z = number_param(params, "offset_z", 0.0);
        target.rotate = number_param(params, "rotate", 0.0);
        return session_.new_sketch(target);
    }
    if (method == "sketch.rectangle") {
        RectangleTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.width = required_number(params, "width");
        target.height = required_number(params, "height");
        target.x = number_param(params, "x", 0.0);
        target.y = number_param(params, "y", 0.0);
        target.centered = bool_param(params, "centered", false);
        target.name_width = string_param(params, "name_width");
        target.name_height = string_param(params, "name_height");
        return session_.rectangle(target);
    }
    if (method == "sketch.circle") {
        CircleTarget target;
        target.document = string_param(params, "document");
        target.sketch = string_param(params, "sketch");
        target.radius = required_number(params, "radius");
        target.x = number_param(params, "x", 0.0);
        target.y = number_param(params, "y", 0.0);
        target.name_radius = string_param(params, "name_radius");
        return session_.circle(target);
    }
    if (method == "pad.new" || method == "pocket.new") {
        const bool cutting = method == "pocket.new";
        ExtrudeTarget target;
        target.document = string_param(params, "document");
        target.body = string_param(params, "body");
        target.sketch = string_param(params, "sketch");
        target.name = string_param(params, "name");
        target.through_all = cutting && bool_param(params, "through_all", false);
        target.length = target.through_all ? number_param(params, "length", 0.0)
                                           : required_number(params, "length");
        target.midplane = bool_param(params, "midplane", false);
        target.reversed = bool_param(params, "reversed", false);
        return cutting ? session_.pocket(target) : session_.pad(target);
    }
    if (method == "pad.length" || method == "pocket.length") {
        const bool cutting = method == "pocket.length";
        return session_.extrude_length(string_param(params, "document"),
                                       string_param(params, cutting ? "pocket" : "pad"),
                                       cutting ? "pocket" : "pad",
                                       required_number(params, "length"));
    }
    if (method == "param.list") {
        return session_.parameters(string_param(params, "document"));
    }
    if (method == "param.set") {
        return session_.set_parameter(string_param(params, "document"),
                                      string_param(params, "name"),
                                      required_number(params, "value"));
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
