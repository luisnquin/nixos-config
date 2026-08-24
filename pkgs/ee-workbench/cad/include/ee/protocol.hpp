#pragma once

#include <functional>
#include <string>

#include "ee/json.hpp"
#include "ee/session.hpp"

namespace ee {

/// Wire version. Bump whenever a method's request or reply shape changes in a
/// way an older `ee` binary would misread.
constexpr long long kProtocol = 12;

/// Which build of this server is answering, opaque to everyone but the packaging
/// that set it. `kProtocol` only moves when the wire shape changes, so two
/// servers can agree on it and still disagree on what a method does — that drift
/// is invisible to a version number but not to this. Empty when built outside
/// Nix, which is how the client's check turns itself off in development.
const char* build_id();

/// A refusal that no method produced: too long a line, a connection the
/// transport is giving up on. Same envelope as any other reply.
std::string protocol_failure(const std::string& code, const std::string& message);

/// The request half of the session: parse a line, run one method, produce one
/// reply line. It knows nothing about sockets, so the blocking server and the
/// GUI event loop answer identically.
class Protocol
{
public:
    /// Returns false when the reply is the last one this connection gets.
    bool handle_line(const std::string& line, std::string& reply);

    /// Re-export the meshes of documents that changed. Takes the GIL itself.
    void refresh_previews();

    /// Called after a request left a followed preview stale. The blocking
    /// server exports at once; the GUI event loop debounces through a timer.
    void on_stale(std::function<void()> handler)
    {
        on_stale_ = std::move(handler);
    }

    bool stopping() const
    {
        return stopping_;
    }

    /// Reported back through `session.status` so a caller can see how long it
    /// has before the server goes away.
    void set_idle_timeout(long long seconds)
    {
        session_.set_idle_timeout(seconds);
    }

    /// True while a document holds geometry nobody saved. The idle timeout
    /// refuses to fire on this: an unattended exit that drops work is worse
    /// than a FreeCAD process that outstays its welcome.
    bool has_unsaved() const;

private:
    json::Value dispatch(const std::string& method, const json::Value* params);

    Session session_;
    std::function<void()> on_stale_;
    bool stopping_ = false;
};

}  // namespace ee
