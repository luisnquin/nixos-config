#pragma once

#include <map>
#include <set>
#include <string>
#include <utility>
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

struct LineTarget
{
    std::string document;
    std::string sketch;
    Slot x1;
    Slot y1;
    Slot x2;
    Slot y2;
};

/// An arc as endpoints plus radius, drawn counter-clockwise from (x1,y1) to
/// (x2,y2). Chosen over centre+angles because every slot is a length the
/// solver can hold with a dimensional constraint - angles are not slots
/// anywhere else in this vocabulary - and over three points because a
/// mid-point is nothing a parameter would ever drive. `large` picks the major
/// arc; the minor one is the default. Bulge direction is a consequence of the
/// endpoint order: swap them to bulge the other way.
struct ArcTarget
{
    std::string document;
    std::string sketch;
    Slot x1;
    Slot y1;
    Slot x2;
    Slot y2;
    Slot radius;
    bool large = false;
};

/// A chain of line segments. Coordinates arrive as slot pairs so any vertex
/// can be parameter-driven; `close` adds the segment back to the start.
struct PolylineTarget
{
    std::string document;
    std::string sketch;
    std::vector<std::pair<Slot, Slot>> points;
    bool close = false;
};

/// Two or more sketches with one solid built through them in order. The first
/// name is the Profile, the rest are Sections in the same order - the order
/// `PartDesign::Loft` threads the shape through. `loft_new` maps to
/// `AdditiveLoft`, `loft_pocket` to `SubtractiveLoft`; both take this one
/// target because they differ only in which way the material goes, same as
/// pad and pocket.
struct LoftTarget
{
    std::string document;
    std::string body;
    std::vector<std::string> sketches;
    std::string name;
    bool ruled = false;
    bool closed = false;
};

/// Pad and pocket differ only in which way the material goes, so they take the
/// same request. `through_all` is a pocket-only depth. `taper` is FreeCAD's
/// own draft angle on the extrude's walls, native to both and needing no edge
/// selection.
struct ExtrudeTarget
{
    std::string document;
    std::string body;
    std::string sketch;
    std::string name;
    Slot length;
    Slot taper;
    bool midplane = false;
    bool reversed = false;
    bool through_all = false;
};

/// Revolve and groove differ only in which way the material goes, mirroring
/// pad and pocket. `axis` names one of the sketch's own in-plane axes - "x"
/// for its local H_Axis, "y" for its V_Axis - which is the only axis every
/// profile already has for free without naming an edge.
struct RevolveTarget
{
    std::string document;
    std::string body;
    std::string sketch;
    std::string name;
    std::string axis = "y";
    Slot angle;
    bool midplane = false;
    bool reversed = false;
};

/// A `PartDesign::Mirrored`: one or more features reflected across a body
/// origin plane. Empty `features` means the body's own tip - the feature the
/// last `pad`/`pocket`/`revolve`/`groove`/`mirror`/`pattern` call just left
/// behind - so a fin sketch plus one call is the whole symmetric pair.
struct MirrorTarget
{
    std::string document;
    std::string body;
    std::string plane;
    std::vector<std::string> features;
    std::string name;
};

/// A `PartDesign::LinearPattern`. `spacing` is the distance between
/// consecutive copies, not the total span, because that is the number that
/// stays right when `count` changes. `LinearPattern` drives a total `Length`
/// internally, so a negative `spacing` degenerates rather than walking the
/// other way - `reversed` is the only way to flip direction.
struct LinearPatternTarget
{
    std::string document;
    std::string body;
    std::string direction = "x";
    std::vector<std::string> features;
    std::string name;
    Slot spacing;
    int count = 2;
    bool reversed = false;
};

/// A `PartDesign::PolarPattern`. `angle` is the total sweep across every copy,
/// matching revolve's own convention, not the per-step angle.
struct PolarPatternTarget
{
    std::string document;
    std::string body;
    std::string axis = "z";
    std::vector<std::string> features;
    std::string name;
    Slot angle;
    int count = 2;
};

/// Which edges of the tip a dressup applies to, composed by AND. Every
/// predicate left at its default (empty axis, zero bound) drops out of the
/// match, so the all-edges default falls out of an all-default struct rather
/// than needing its own case. Axes are "x"/"y"/"z"; empty means unset.
struct EdgeSelection
{
    std::string parallel;
    std::string near_min;
    std::string near_max;
    double longer_than = 0.0;
    bool has_longer_than = false;
    double shorter_than = 0.0;
    bool has_shorter_than = false;
};

/// A `PartDesign::Fillet` over edges resolved from geometry, never a raw
/// `EdgeN` name - those are FreeCAD's topological naming and shift under any
/// upstream change, so they must never appear in a request. Empty `features`
/// means the body's own tip; `DressUp::Base` is a single link, so more than
/// one name is a refusal rather than a pick of the first.
struct FilletTarget
{
    std::string document;
    std::string body;
    std::vector<std::string> features;
    std::string name;
    Slot radius;
    EdgeSelection selection;
};

/// A `PartDesign::Chamfer`, same edge resolution and single-feature targeting
/// as fillet. `angle` at its default (0) is "Equal distance" mode on `size`
/// alone, matching how a zero `taper` already means "no taper" elsewhere in
/// this vocabulary; any other value switches to "Distance and Angle" mode.
struct ChamferTarget
{
    std::string document;
    std::string body;
    std::vector<std::string> features;
    std::string name;
    Slot size;
    Slot angle;
    EdgeSelection selection;
};

/// A `PartDesign::Boolean` folding one or more tool bodies into a base body's
/// own chain. The tool bodies are not deleted afterwards - a boolean only
/// reparents them under itself, so their name and feature history stay
/// addressable - but `top_level_solids`/`preview_target` stop counting them as
/// shapes of their own, which is what turns "two bodies" from an
/// ambiguous-shape refusal into one combined result. `operation` is
/// "union"/"cut"/"intersect".
struct BooleanTarget
{
    std::string document;
    std::string base;
    std::vector<std::string> tool;
    std::string operation;
    std::string name;
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
    std::string body;
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
    json::Value line(const LineTarget& target);
    json::Value arc(const ArcTarget& target);
    json::Value polyline(const PolylineTarget& target);
    json::Value pad(const ExtrudeTarget& target);
    json::Value pocket(const ExtrudeTarget& target);
    json::Value revolve(const RevolveTarget& target);
    json::Value groove(const RevolveTarget& target);
    /// `AdditiveLoft` through `target.sketches` in order.
    json::Value loft_new(const LoftTarget& target);
    /// `SubtractiveLoft` through `target.sketches` in order.
    json::Value loft_pocket(const LoftTarget& target);
    json::Value mirror(const MirrorTarget& target);
    json::Value pattern_linear(const LinearPatternTarget& target);
    json::Value pattern_polar(const PolarPatternTarget& target);
    json::Value fillet(const FilletTarget& target);
    json::Value chamfer(const ChamferTarget& target);
    /// Fold `target.tool` bodies into `target.base`'s own chain. Fails, and
    /// leaves both bodies untouched, if the result comes out with no solid at
    /// all - an intersect of disjoint bodies is exactly that case, and FreeCAD
    /// itself does not treat an empty boolean result as an error.
    json::Value body_boolean(const BooleanTarget& target);
    /// Point an existing slot at a parameter, or back at a literal.
    json::Value set_slot(const SlotTarget& target);
    json::Value parameters(const std::string& document) const;
    json::Value declare_parameter(const ParamTarget& target);
    /// The same expression path `declare_parameter` binds a name to, without
    /// leaving a parameter behind: a throwaway registry row is expressed,
    /// recomputed and removed. This is how a geometry slot reads a unit-bearing
    /// quantity like "5 cm" or "2 m / 3" - the unit grammar only exists inside
    /// an expression, so a bare number never reaches this.
    double evaluate_quantity(const std::string& document, const std::string& expression);
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
