#include "ee/listener.hpp"

#include <cerrno>
#include <cstring>
#include <stdexcept>
#include <system_error>
#include <utility>

#include <fcntl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

namespace ee {
namespace {

std::system_error errno_error(const std::string& what)
{
    return std::system_error(errno, std::generic_category(), what);
}

}  // namespace

Listener::Listener(std::string path)
    : path_(std::move(path))
{
}

Listener::~Listener()
{
    if (fd_ >= 0) {
        ::close(fd_);
    }
    if (bound_) {
        ::unlink(path_.c_str());
    }
}

void Listener::open(bool nonblocking)
{
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    if (path_.size() >= sizeof(address.sun_path)) {
        throw std::runtime_error("socket path is too long: " + path_);
    }
    std::memcpy(address.sun_path, path_.c_str(), path_.size());

    // A leftover socket file from a killed server is only safe to remove once a
    // connect proves nobody is listening on it.
    const int probe = ::socket(AF_UNIX, SOCK_STREAM, 0);
    if (probe >= 0) {
        const int connected =
            ::connect(probe, reinterpret_cast<sockaddr*>(&address), sizeof(address));
        ::close(probe);
        if (connected == 0) {
            throw std::runtime_error("another server already owns " + path_);
        }
        ::unlink(path_.c_str());
    }

    int flags = SOCK_STREAM | SOCK_CLOEXEC;
    if (nonblocking) {
        flags |= SOCK_NONBLOCK;
    }
    fd_ = ::socket(AF_UNIX, flags, 0);
    if (fd_ < 0) {
        throw errno_error("socket");
    }

    const mode_t previous = ::umask(0177);
    const int bound = ::bind(fd_, reinterpret_cast<sockaddr*>(&address), sizeof(address));
    ::umask(previous);
    if (bound < 0) {
        throw errno_error("bind " + path_);
    }
    bound_ = true;

    if (::listen(fd_, 8) < 0) {
        throw errno_error("listen " + path_);
    }
}

}  // namespace ee
