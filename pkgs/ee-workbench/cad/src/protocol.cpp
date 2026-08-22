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
        return session_.open_document(string_param(params, "path"));
    }
    if (method == "document.recompute") {
        return session_.recompute(string_param(params, "document"));
    }
    if (method == "document.save") {
        return session_.save(string_param(params, "document"), string_param(params, "path"));
    }
    if (method == "document.inspect") {
        return session_.inspect(string_param(params, "document"));
    }
    if (method == "body.new") {
        return session_.new_body(string_param(params, "document"), string_param(params, "name"));
    }
    if (method == "sketch.new") {
        return session_.new_sketch(string_param(params, "document"),
                                   string_param(params, "body"),
                                   string_param(params, "plane"),
                                   string_param(params, "name"));
    }
    if (method == "sketch.rectangle") {
        return session_.rectangle(string_param(params, "document"),
                                  string_param(params, "sketch"),
                                  required_number(params, "width"),
                                  required_number(params, "height"));
    }
    if (method == "pad.new") {
        return session_.pad(string_param(params, "document"),
                            string_param(params, "body"),
                            string_param(params, "sketch"),
                            required_number(params, "length"),
                            bool_param(params, "midplane", false),
                            bool_param(params, "reversed", false),
                            string_param(params, "name"));
    }
    if (method == "pad.length") {
        return session_.pad_length(string_param(params, "document"),
                                   string_param(params, "pad"),
                                   required_number(params, "length"));
    }
    if (method == "preview.export") {
        PreviewRequest request;
        request.document = string_param(params, "document");
        request.object = string_param(params, "object");
        request.path = string_param(params, "path");
        request.tessellation.deflection =
            number_param(params, "deflection", request.tessellation.deflection);
        request.tessellation.angular =
            number_param(params, "angular", request.tessellation.angular);
        request.follow = bool_param(params, "follow", true);
        return session_.preview(request);
    }
    throw Error{"unknown-method", "no method named " + method};
}

}  // namespace ee
