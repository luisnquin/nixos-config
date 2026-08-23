#pragma once

#include <map>
#include <set>
#include <string>
#include <vector>

#include "ee/json.hpp"
#include "ee/mesh.hpp"
#include "ee/params.hpp"

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
/// the whole reason a body no longer has to start at the global origin. Every
/// one of them is a slot, because the placement is what a second sketch has to
/// follow when a dimension moves and a constraint alone cannot carry it.
struct SketchTarget
{
    std::string document;
    std::string body;
    std::string plane;
    std::string name;
    Slot offset_x;
    Slot offset_y;
    Slot offset_z;
    Slot rotate;
};

/// `x` and `y` place the rectangle's reference point in sketch coordinates;
/// `centered` makes that reference point the centre instead of the lower left
/// corner. The four dimensions are named after their slots, so every one of
/// them can be driven by a parameter later even when it was drawn as a literal.
struct RectangleTarget
{
    std::string document;
    std::string sketch;
    Slot width;
    Slot height;
    Slot x;
    Slot y;
    bool centered = false;
};

struct CircleTarget
{
    std::string document;
    std::string sketch;
    Slot radius;
    Slot x;
    Slot y;
};

/// Pad and pocket differ only in which way the material goes, so they take the
/// same request. `through_all` is a pocket-only depth.
struct ExtrudeTarget
{
    std::string document;
    std::string body;
    std::string sketch;
    std::string name;
    Slot length;
    bool midplane = false;
    bool reversed = false;
    bool through_all = false;
};

/// One numeric slot on one object, after the fact. `kind` is how an unnamed
/// object is resolved - the only pad, the newest sketch - and `unbind` is the
/// deliberate half of the refusal that stops a literal from silently replacing
/// a parameter someone is driving.
struct SlotTarget
{
    std::string document;
    std::string object;
    std::string kind;
    std::string slot;
    Slot value;
    bool unbind = false;
};

/// What to take back out of the model, and whether to only say what that would
/// do. `dry_run` decides nothing but that: the same plan is computed either
/// way, so the preview cannot describe an edit the apply would not make.
struct RemovalTarget
{
    std::string document;
    std::string feature;
    bool dry_run = false;
};

/// A parameter's definition: a literal, or an expression over its siblings.
/// `must_be_new` separates `param new` from `param set`, so neither one can
/// quietly do the other's job.
struct ParamTarget
{
    std::string document;
    std::string name;
    std::string expression;
    double value = 0.0;
    bool must_be_new = false;
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
    /// Point an existing slot at a parameter, or back at a literal.
    json::Value set_slot(const SlotTarget& target);
    json::Value parameters(const std::string& document) const;
    json::Value declare_parameter(const ParamTarget& target);
    /// `force` frees every slot the parameter drives instead of refusing, and
    /// the reply names each one: turning N relationships into literals is the
    /// kind of state change the rest of this refuses to make silently.
    json::Value remove_parameter(const std::string& document,
                                 const std::string& name,
                                 bool force);
    /// Take a feature out of a body's tree, or a sketch out of the document.
    json::Value remove_feature(const RemovalTarget& target);
    json::Value preview(const PreviewRequest& request);
    json::Value render(const RenderTarget& target);
    json::Value recompute(const std::string& document);
    json::Value save(const std::string& document, const std::string& path);
    json::Value inspect(const std::string& document, bool features) const;

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
