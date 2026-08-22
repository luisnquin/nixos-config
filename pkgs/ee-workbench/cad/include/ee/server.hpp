#pragma once

#include <csignal>
#include <string>

#include "ee/listener.hpp"
#include "ee/protocol.hpp"

namespace ee {

/// One connection at a time, one request at a time: FreeCAD's document graph is
/// not thread-safe and every method here mutates it. Used by the headless
/// binary, where there is no event loop to share.
class Server
{
public:
    explicit Server(std::string socket_path);

    Server(const Server&) = delete;
    Server& operator=(const Server&) = delete;

    void listen();
    void run();

    const std::string& socket_path() const
    {
        return listener_.path();
    }

private:
    void serve(int client);

    Listener listener_;
    Protocol protocol_;
};

/// Set from the signal handler; the accept loop leaves as soon as it notices.
extern volatile sig_atomic_t g_interrupted;

}  // namespace ee
