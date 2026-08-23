#pragma once

#include <string>
#include <vector>

#include "ee/json.hpp"

namespace App {
class Document;
class DocumentObject;
class VarSet;
}  // namespace App

namespace ee {

/// A numeric slot as the wire spells it: a literal, or the name of the
/// parameter driving it. Every dimension the CLI can set is one of these in
/// both directions - a slot created as a literal can be bound afterwards and a
/// bound one can be freed - because a model whose values are parametric but
/// whose parameterization is write-once is only half the fix.
struct Slot
{
    double value = 0.0;
    std::string parameter;

    bool bound() const
    {
        return !parameter.empty();
    }
};

/// The one place arithmetic is allowed to live. Parameters may be expressions
/// over other parameters; geometry only ever holds a constant or a name. That
/// is what makes `param list` the whole dependency graph rather than an index
/// into one, and it is why a slot never carries an expression of its own.
namespace params {

/// The VarSet every document keeps its parameters in. Named, not searched by
/// type: a document may hold VarSets that are none of our business.
constexpr const char* kRegistry = "Parameters";

App::VarSet* find(App::Document& doc);
App::VarSet& ensure(App::Document& doc);

/// Refuses anything FreeCAD's grammar would not read back as a bare name.
/// Leading digits go with it: a slot takes a literal or a name without a sigil
/// to tell them apart, which only stays unambiguous while no name is a number.
void require_name(const std::string& name);

/// Every parameter in the registry, in declaration order.
std::vector<std::string> names(App::Document& doc);

bool exists(App::Document& doc, const std::string& name);
double value_of(App::Document& doc, const std::string& name);

/// The number to write into the slot now. A bound slot still needs one: the
/// expression is only evaluated by the next recompute, and the constraint
/// solver wants a datum before then.
double resolve(App::Document& doc, const Slot& slot);

/// Points `path` on `object` at a parameter, or frees it when the slot holds a
/// literal. Freeing leaves the last computed number behind rather than zero,
/// so unbinding never moves the geometry by itself.
void apply(App::Document& doc,
           App::DocumentObject& object,
           const std::string& path,
           const Slot& slot);

/// Sets a parameter to a literal or to an expression over its siblings.
/// `expression` is the text as typed, without the leading `=`.
void declare(App::VarSet& registry, const std::string& name, double value);
void express(App::VarSet& registry, const std::string& name, const std::string& expression);

/// What is bound to `path`, read out of the expression engine rather than
/// remembered: a document opened from disk answers the same as one just built.
/// `parameter` is set only when the expression is exactly a reference to one.
struct Binding
{
    std::string expression;
    std::string parameter;
};

Binding binding_of(const App::DocumentObject& object, const std::string& path);

/// `{"value": 6.0, "parameter": "plate_t"}` - the readback shape every numeric
/// slot reports, so no caller has to infer from a number whether it will move.
json::Value slot_json(const App::DocumentObject& object, const std::string& path, double value);

/// Every slot in the document pointing at `name`, as `{object, slot}` pairs.
json::Value drives(App::Document& doc, const std::string& name);

/// The parameters whose own expression reads a property of `object`. Geometry
/// never computes, so this is the only direction a reference can run that
/// removing an object would break, and relinking cannot repair it: the
/// arithmetic named a thing that stops existing.
std::vector<std::string> following(App::Document& doc, const App::DocumentObject& object);

}  // namespace params
}  // namespace ee
