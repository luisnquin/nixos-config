#pragma once

#include <list>
#include <memory>
#include <string>

#include <QObject>

#include "ee/listener.hpp"
#include "ee/protocol.hpp"

class QSocketNotifier;
class QTimer;

namespace ee {

/// The same protocol served from inside FreeCAD's GUI process. Accepting,
/// reading and writing are all socket notifiers on the Qt event loop, so every
/// FreeCAD mutation happens on the thread that owns the document and its view
/// providers, and no phase of the exchange can park the interface.
class QtServer: public QObject
{
public:
    QtServer(std::string socket_path, QObject* parent);
    ~QtServer() override;

    void start();

    const std::string& socket_path() const
    {
        return listener_.path();
    }

private:
    struct Connection
    {
        ~Connection();

        int fd = -1;
        std::unique_ptr<QSocketNotifier> readable;
        std::unique_ptr<QSocketNotifier> writable;
        std::string in;
        std::string out;
        bool closing = false;
    };

    void accept_ready();
    void read_ready(Connection& connection);
    void flush(Connection& connection);
    void drop(Connection& connection);

    Listener listener_;
    Protocol protocol_;
    std::unique_ptr<QSocketNotifier> incoming_;
    QTimer* debounce_ = nullptr;
    std::list<std::unique_ptr<Connection>> connections_;
    /// Connections dropped from inside their own notifier callback. Qt touches
    /// the notifier after the callback returns, so they die one turn later.
    std::list<std::unique_ptr<Connection>> closed_;
};

}  // namespace ee
