#pragma once

#include <string>

namespace ee {

/// A bound Unix socket. Whoever binds it owns the file: a listener that refused
/// to start must never remove the socket of the session that is already there.
class Listener
{
public:
    explicit Listener(std::string path);
    ~Listener();

    Listener(const Listener&) = delete;
    Listener& operator=(const Listener&) = delete;

    /// Binds and listens. `nonblocking` is what lets an event loop own the
    /// accept instead of a thread parked in accept(2).
    void open(bool nonblocking);

    int fd() const
    {
        return fd_;
    }

    const std::string& path() const
    {
        return path_;
    }

private:
    std::string path_;
    int fd_ = -1;
    bool bound_ = false;
};

}  // namespace ee
