#pragma once

#include <functional>
#include <string>

#include "ee/json.hpp"
#include "ee/session.hpp"

namespace ee {

/// Wire version. Bump whenever a method's request or reply shape changes in a
/// way an older `ee` binary would misread.
constexpr long long kProtocol = 3;

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

private:
    json::Value dispatch(const std::string& method, const json::Value* params);

    Session session_;
    std::function<void()> on_stale_;
    bool stopping_ = false;
};

}  // namespace ee
