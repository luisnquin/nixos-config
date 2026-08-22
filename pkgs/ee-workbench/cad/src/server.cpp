#include "ee/server.hpp"

#include <cerrno>
#include <string>
#include <system_error>
#include <utility>

#include <sys/socket.h>
#include <unistd.h>

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

}  // namespace

Server::Server(std::string socket_path)
    : listener_(std::move(socket_path))
{
    // Nothing else runs here, so a stale mesh can be rewritten immediately.
    protocol_.on_stale([this] { protocol_.refresh_previews(); });
}

void Server::listen()
{
    listener_.open(false);
}

void Server::run()
{
    while (g_interrupted == 0 && !protocol_.stopping()) {
        const int client = ::accept(listener_.fd(), nullptr, nullptr);
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

    while (!protocol_.stopping()) {
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
            const bool keep = protocol_.handle_line(line, reply);
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
                protocol_failure("line-too-long", "request exceeds 1 MiB") + "\n";
            // Best effort: the connection is being dropped either way.
            const ssize_t sent = ::write(client, reply.data(), reply.size());
            static_cast<void>(sent);
            return;
        }
    }
}

}  // namespace ee
