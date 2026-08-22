#include "ee/qtserver.hpp"

#include <cerrno>
#include <string>
#include <utility>

#include <sys/socket.h>
#include <unistd.h>

#include <QSocketNotifier>
#include <QTimer>

namespace ee {
namespace {

constexpr std::size_t kMaxLine = 1u << 20;

/// Long enough to collapse a burst of commands into one export, short enough
/// that a viewer feels the change as immediate.
constexpr int kDebounceMs = 250;

}  // namespace

QtServer::Connection::~Connection()
{
    readable.reset();
    writable.reset();
    if (fd >= 0) {
        ::close(fd);
    }
}

QtServer::QtServer(std::string socket_path, QObject* parent)
    : QObject(parent)
    , listener_(std::move(socket_path))
{
}

QtServer::~QtServer() = default;

void QtServer::start()
{
    listener_.open(true);

    debounce_ = new QTimer(this);
    debounce_->setSingleShot(true);
    debounce_->setInterval(kDebounceMs);
    QObject::connect(debounce_, &QTimer::timeout, this, [this] { protocol_.refresh_previews(); });
    protocol_.on_stale([this] { debounce_->start(); });

    incoming_ = std::make_unique<QSocketNotifier>(listener_.fd(), QSocketNotifier::Read);
    QObject::connect(incoming_.get(), &QSocketNotifier::activated, this, [this] { accept_ready(); });
}

void QtServer::accept_ready()
{
    while (true) {
        const int client = ::accept4(listener_.fd(), nullptr, nullptr,
                                     SOCK_NONBLOCK | SOCK_CLOEXEC);
        if (client < 0) {
            return;
        }

        auto connection = std::make_unique<Connection>();
        connection->fd = client;
        Connection& handle = *connection;
        connections_.push_back(std::move(connection));

        handle.readable = std::make_unique<QSocketNotifier>(client, QSocketNotifier::Read);
        QObject::connect(handle.readable.get(), &QSocketNotifier::activated, this,
                         [this, &handle] { read_ready(handle); });

        handle.writable = std::make_unique<QSocketNotifier>(client, QSocketNotifier::Write);
        handle.writable->setEnabled(false);
        QObject::connect(handle.writable.get(), &QSocketNotifier::activated, this,
                         [this, &handle] { flush(handle); });
    }
}

void QtServer::read_ready(Connection& connection)
{
    char buffer[4096];

    while (true) {
        const ssize_t got = ::read(connection.fd, buffer, sizeof(buffer));
        if (got < 0) {
            if (errno == EINTR) {
                continue;
            }
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                break;
            }
            drop(connection);
            return;
        }
        if (got == 0) {
            drop(connection);
            return;
        }
        connection.in.append(buffer, static_cast<std::size_t>(got));
    }

    std::size_t start = 0;
    for (std::size_t end = connection.in.find('\n', start); end != std::string::npos;
         end = connection.in.find('\n', start)) {
        const std::string line = connection.in.substr(start, end - start);
        start = end + 1;

        std::string reply;
        const bool keep = protocol_.handle_line(line, reply);
        connection.out += reply;
        connection.out.push_back('\n');
        if (!keep) {
            connection.closing = true;
            break;
        }
    }
    connection.in.erase(0, start);

    if (!connection.closing && connection.in.size() > kMaxLine) {
        connection.out += protocol_failure("line-too-long", "request exceeds 1 MiB");
        connection.out.push_back('\n');
        connection.closing = true;
    }

    flush(connection);
}

void QtServer::flush(Connection& connection)
{
    while (!connection.out.empty()) {
        const ssize_t sent = ::write(connection.fd, connection.out.data(), connection.out.size());
        if (sent < 0) {
            if (errno == EINTR) {
                continue;
            }
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                connection.writable->setEnabled(true);
                return;
            }
            drop(connection);
            return;
        }
        connection.out.erase(0, static_cast<std::size_t>(sent));
    }

    connection.writable->setEnabled(false);
    if (connection.closing) {
        drop(connection);
    }
}

void QtServer::drop(Connection& connection)
{
    connection.readable->setEnabled(false);
    connection.writable->setEnabled(false);

    for (auto it = connections_.begin(); it != connections_.end(); ++it) {
        if (it->get() != &connection) {
            continue;
        }
        closed_.splice(closed_.end(), connections_, it);
        break;
    }
    QTimer::singleShot(0, this, [this] { closed_.clear(); });
}

}  // namespace ee
