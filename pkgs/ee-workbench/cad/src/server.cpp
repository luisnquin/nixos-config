#include "ee/server.hpp"

#include <cerrno>
#include <cstring>
#include <stdexcept>
#include <string>
#include <system_error>
#include <utility>

#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#include <Base/Exception.h>
#include <Base/Interpreter.h>

namespace ee {

volatile sig_atomic_t g_interrupted = 0;

namespace {

/// Long enough for any legitimate request, short enough that a stuck or hostile
/// client cannot make the server allocate without bound.
constexpr std::size_t kMaxLine = 1u << 20;

std::system_error errno_error(const std::string& what)
{
    return std::system_error(errno, std::generic_category(), what);
}

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

Server::Server(std::string socket_path)
    : socket_path_(std::move(socket_path))
{
}

Server::~Server()
{
    if (listener_ >= 0) {
        ::close(listener_);
    }
    if (bound_) {
        ::unlink(socket_path_.c_str());
    }
}

void Server::listen()
{
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    if (socket_path_.size() >= sizeof(address.sun_path)) {
        throw std::runtime_error("socket path is too long: " + socket_path_);
    }
    std::memcpy(address.sun_path, socket_path_.c_str(), socket_path_.size());

    // A leftover socket file from a killed server is only safe to remove once a
    // connect proves nobody is listening on it.
    const int probe = ::socket(AF_UNIX, SOCK_STREAM, 0);
    if (probe >= 0) {
        const int connected =
            ::connect(probe, reinterpret_cast<sockaddr*>(&address), sizeof(address));
        ::close(probe);
        if (connected == 0) {
            throw std::runtime_error("another server already owns " + socket_path_);
        }
        ::unlink(socket_path_.c_str());
    }

    listener_ = ::socket(AF_UNIX, SOCK_STREAM, 0);
    if (listener_ < 0) {
        throw errno_error("socket");
    }

    const mode_t previous = ::umask(0177);
    const int bound = ::bind(listener_, reinterpret_cast<sockaddr*>(&address), sizeof(address));
    ::umask(previous);
    if (bound < 0) {
        throw errno_error("bind " + socket_path_);
    }
    bound_ = true;

    if (::listen(listener_, 8) < 0) {
        throw errno_error("listen " + socket_path_);
    }
}

void Server::run()
{
    while (g_interrupted == 0 && !stopping_) {
        const int client = ::accept(listener_, nullptr, nullptr);
        if (client < 0) {
            if (errno == EINTR) {
                continue;
            }
            throw errno_error("accept");
        }
        serve(client);
        ::close(client);
    }
}

void Server::serve(int client)
{
    std::string pending;
    char buffer[4096];

    while (!stopping_) {
        const ssize_t got = ::read(client, buffer, sizeof(buffer));
        if (got < 0) {
            if (errno == EINTR) {
                if (g_interrupted != 0) {
                    return;
                }
                continue;
            }
            return;
        }
        if (got == 0) {
            return;
        }

        pending.append(buffer, static_cast<std::size_t>(got));

        std::size_t start = 0;
        for (std::size_t end = pending.find('\n', start); end != std::string::npos;
             end = pending.find('\n', start)) {
            std::string line = pending.substr(start, end - start);
            start = end + 1;

            std::string reply;
            const bool keep = handle_line(line, reply);
            reply.push_back('\n');

            std::size_t written = 0;
            while (written < reply.size()) {
                const ssize_t sent = ::write(client, reply.data() + written, reply.size() - written);
                if (sent < 0) {
                    if (errno == EINTR) {
                        continue;
                    }
                    return;
                }
                written += static_cast<std::size_t>(sent);
            }
            if (!keep) {
                return;
            }
        }
        pending.erase(0, start);

        if (pending.size() > kMaxLine) {
            const std::string reply =
                failure(json::Value(), "line-too-long", "request exceeds 1 MiB") + "\n";
            ::write(client, reply.data(), reply.size());
            return;
        }
    }
}

bool Server::handle_line(const std::string& line, std::string& reply)
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
                            "server speaks protocol " + std::to_string(kProtocol) + ", client sent " +
                                std::to_string(*version));
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
    }
    catch (const Base::Exception& error) {
        reply = failure(id, "freecad", error.what());
    }
    catch (const std::exception& error) {
        reply = failure(id, "internal", error.what());
    }
    return true;
}

json::Value Server::dispatch(const std::string& method, const json::Value* params)
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
    throw Error{"unknown-method", "no method named " + method};
}

}  // namespace ee
