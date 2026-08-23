#pragma once

#include <map>
#include <set>
#include <string>
#include <vector>

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

/// An offscreen picture of the model. With no `object` every top level solid
/// in the document is drawn, because the question a render answers is what the
/// whole thing looks like.
struct RenderTarget
{
    std::string document;
    std::string object;
    std::string path;
    std::string view = "iso";
    int width = 900;
    int height = 700;
    Tessellation tessellation;
};

/// Where a sketch sits. `plane` picks the origin plane, the offset moves it off
/// that plane, and `rotate` spins it about its own normal - together they are
/// the whole reason a body no longer has to start at the global origin.
struct SketchTarget
{
    std::string document;
    std::string body;
    std::string plane;
    std::string name;
    double offset_x = 0.0;
    double offset_y = 0.0;
    double offset_z = 0.0;
    double rotate = 0.0;
};

/// `x` and `y` place the rectangle's reference point in sketch coordinates;
/// `centered` makes that reference point the centre instead of the lower left
/// corner. Naming a dimension makes it addressable by `param` afterwards.
struct RectangleTarget
{
    std::string document;
    std::string sketch;
    double width = 0.0;
    double height = 0.0;
    double x = 0.0;
    double y = 0.0;
    bool centered = false;
    std::string name_width;
    std::string name_height;
};

struct CircleTarget
{
    std::string document;
    std::string sketch;
    double radius = 0.0;
    double x = 0.0;
    double y = 0.0;
    std::string name_radius;
};

/// Pad and pocket differ only in which way the material goes, so they take the
/// same request. `through_all` is a pocket-only depth.
struct ExtrudeTarget
{
    std::string document;
    std::string body;
    std::string sketch;
    std::string name;
    double length = 0.0;
    bool midplane = false;
    bool reversed = false;
    bool through_all = false;
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
    json::Value new_sketch(const SketchTarget& target);
    json::Value rectangle(const RectangleTarget& target);
    json::Value circle(const CircleTarget& target);
    json::Value pad(const ExtrudeTarget& target);
    json::Value pocket(const ExtrudeTarget& target);
    /// Retarget an existing pad or pocket's extrude length and recompute.
    json::Value extrude_length(const std::string& document,
                               const std::string& feature,
                               const std::string& kind,
                               double length);
    json::Value parameters(const std::string& document) const;
    json::Value set_parameter(const std::string& document,
                              const std::string& name,
                              double value);
    json::Value preview(const PreviewRequest& request);
    json::Value render(const RenderTarget& target);
    json::Value recompute(const std::string& document);
    json::Value save(const std::string& document, const std::string& path);
    json::Value inspect(const std::string& document) const;

    /// A document mutated since the last export and has a followed preview.
    bool preview_pending() const
    {
        return !dirty_.empty();
    }

    /// Documents holding geometry this session created and nobody saved.
    /// Losing one costs real work, so the idle timeout refuses to fire while
    /// any exist.
    std::vector<std::string> unsaved_documents() const;

    /// Re-exports every followed preview whose document changed. Errors are
    /// reported per document instead of failing the batch: a broken recompute
    /// must not take down the connection that could fix it.
    json::Value refresh_previews();

    void set_idle_timeout(long long seconds)
    {
        idle_timeout_ = seconds;
    }

private:
    struct Followed
    {
        std::string object;
        std::string path;
        Tessellation tessellation;
    };

    void mark_dirty(const std::string& document);
    /// Both "a preview is stale" and "this document must not be lost".
    void mark_changed(const std::string& document);

    std::map<std::string, Followed> followed_;
    std::set<std::string> dirty_;
    std::set<std::string> unsaved_;
    long long idle_timeout_ = 0;
};

}  // namespace ee
