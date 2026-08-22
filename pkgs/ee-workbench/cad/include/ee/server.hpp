#pragma once

#include <csignal>
#include <string>

#include "ee/json.hpp"
#include "ee/session.hpp"

namespace ee {

/// Wire version. Bump whenever a method's request or reply shape changes in a
/// way an older `ee` binary would misread.
constexpr long long kProtocol = 2;

/// One connection at a time, one request at a time: FreeCAD's document graph is
/// not thread-safe and every method here mutates it.
class Server
{
public:
    explicit Server(std::string socket_path);
    ~Server();

    Server(const Server&) = delete;
    Server& operator=(const Server&) = delete;

    void listen();
    void run();

    const std::string& socket_path() const
    {
        return socket_path_;
    }

private:
    void serve(int client);
    bool handle_line(const std::string& line, std::string& reply);
    json::Value dispatch(const std::string& method, const json::Value* params);

    std::string socket_path_;
    int listener_ = -1;
    /// Only a server that bound the socket may remove it; refusing to start
    /// must never delete the running server's socket.
    bool bound_ = false;
    bool stopping_ = false;
    Session session_;
};

/// Set from the signal handler; the accept loop leaves as soon as it notices.
extern volatile sig_atomic_t g_interrupted;

}  // namespace ee
