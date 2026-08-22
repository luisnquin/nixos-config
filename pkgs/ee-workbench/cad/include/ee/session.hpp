#pragma once

#include <map>
#include <set>
#include <string>

#include "ee/json.hpp"
#include "ee/mesh.hpp"

namespace ee {

/// A refusal the client can branch on: `code` is part of the wire contract,
/// `message` is for a human reading the terminal.
struct Error
{
    std::string code;
    std::string message;
};

struct PreviewRequest
{
    std::string document;
    std::string object;
    std::string path;
    Tessellation tessellation;
    /// Keep re-exporting after every later recompute of the same document.
    bool follow = true;
};

/// Every method drives the real FreeCAD document graph in this process.
/// FreeCAD's `App::Application` singleton is the state; this type only holds
/// the rules about which document and which object a request means, plus which
/// documents are being mirrored to a preview mesh.
class Session
{
public:
    json::Value status() const;
    json::Value new_document(const std::string& name);
    json::Value open_document(const std::string& path);
    json::Value new_body(const std::string& document, const std::string& name);
    json::Value new_sketch(const std::string& document,
                           const std::string& body,
                           const std::string& plane,
                           const std::string& name);
    json::Value rectangle(const std::string& document,
                          const std::string& sketch,
                          double width,
                          double height);
    json::Value pad(const std::string& document,
                    const std::string& body,
                    const std::string& sketch,
                    double length,
                    bool midplane,
                    bool reversed,
                    const std::string& name);
    json::Value pad_length(const std::string& document, const std::string& pad, double length);
    json::Value preview(const PreviewRequest& request);
    json::Value recompute(const std::string& document);
    json::Value save(const std::string& document, const std::string& path);
    json::Value inspect(const std::string& document) const;

    /// A document mutated since the last export and has a followed preview.
    bool preview_pending() const
    {
        return !dirty_.empty();
    }

    /// Re-exports every followed preview whose document changed. Errors are
    /// reported per document instead of failing the batch: a broken recompute
    /// must not take down the connection that could fix it.
    json::Value refresh_previews();

private:
    struct Followed
    {
        std::string object;
        std::string path;
        Tessellation tessellation;
    };

    void mark_dirty(const std::string& document);

    std::map<std::string, Followed> followed_;
    std::set<std::string> dirty_;
};

}  // namespace ee
