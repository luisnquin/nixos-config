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

    /// Seconds with no connection after which the server exits on its own. Zero
    /// keeps it alive forever. It is what makes an auto-spawned session
    /// disposable: nobody has to remember to stop it.
    void set_idle_timeout(long long seconds);

    /// True when `run` returned because the idle timeout expired rather than
    /// because someone asked it to stop.
    bool timed_out() const
    {
        return timed_out_;
    }

    const std::string& socket_path() const
    {
        return listener_.path();
    }

private:
    void serve(int client);

    Listener listener_;
    Protocol protocol_;
    long long idle_timeout_ = 0;
    bool timed_out_ = false;
};

/// Set from the signal handler; the accept loop leaves as soon as it notices.
extern volatile sig_atomic_t g_interrupted;

}  // namespace ee
