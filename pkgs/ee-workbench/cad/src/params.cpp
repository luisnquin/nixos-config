#include "ee/params.hpp"

#include <cctype>
#include <map>
#include <memory>
#include <typeinfo>

#include <App/Document.h>
#include <App/DocumentObject.h>
#include <App/Expression.h>
#include <App/ObjectIdentifier.h>
#include <App/PropertyStandard.h>
#include <App/VarSet.h>
#include <Base/Exception.h>

#include "ee/session.hpp"

namespace ee::params {
namespace {

/// Unitless on purpose. `App::PropertyLength` carries millimetres, and FreeCAD
/// refuses `head_len / 2 - 15` on it with a unit mismatch: the dimensionless
/// literal has nothing to subtract from. The expression grammar we want is what
/// picks the type, so the CLI's existing "numbers are millimetres" contract
/// stays a contract rather than becoming a unit.
constexpr const char* kType = "App::PropertyFloat";

App::PropertyFloat* property_of(App::VarSet& registry, const std::string& name)
{
    return dynamic_cast<App::PropertyFloat*>(registry.getPropertyByName(name.c_str()));
}

App::PropertyFloat& require_property(App::Document& doc, const std::string& name)
{
    App::VarSet* registry = find(doc);
    App::PropertyFloat* property =
        registry == nullptr ? nullptr : property_of(*registry, name);
    if (property == nullptr) {
        throw Error{"unknown-parameter",
                    "no parameter named " + name + " in " + doc.getName() +
                        ": `param list` names them, `param new " + name +
                        " <value>` declares one"};
    }
    return *property;
}

App::ObjectIdentifier identifier(App::DocumentObject& object, const std::string& path)
{
    try {
        return App::ObjectIdentifier::parse(&object, path);
    }
    catch (const Base::Exception& error) {
        throw Error{"unknown-slot", std::string("no slot ") + path + " on " +
                                        object.getNameInDocument() + ": " + error.what()};
    }
}

/// FreeCAD's own refusals arrive as Base::Exception with a usable message - a
/// cycle says so in as many words - so they are relabelled rather than
/// reworded.
void set_expression(App::DocumentObject& object,
                    const App::ObjectIdentifier& path,
                    const std::string& text)
{
    try {
        if (text.empty()) {
            object.setExpression(path, std::shared_ptr<App::Expression>());
            return;
        }
        object.setExpression(path, std::shared_ptr<App::Expression>(
                                       App::Expression::parse(&object, text)));
    }
    catch (const Base::Exception& error) {
        throw Error{"invalid-expression", error.what()};
    }
}

/// True when `identifier` names the parameter `name` in `registry`: qualified
/// from anywhere else in the document, bare from inside the registry itself.
bool refers_to(const App::ObjectIdentifier& identifier,
               const App::VarSet& registry,
               const std::string& name)
{
    if (identifier.getPropertyName() != name) {
        return false;
    }
    const std::string owner = identifier.getDocumentObjectName().getString();
    return owner.empty() || owner == registry.getNameInDocument();
}

std::string leading_dot_stripped(std::string path)
{
    if (!path.empty() && path.front() == '.') {
        path.erase(0, 1);
    }
    return path;
}

}  // namespace

App::VarSet* find(App::Document& doc)
{
    return dynamic_cast<App::VarSet*>(doc.getObject(kRegistry));
}

App::VarSet& ensure(App::Document& doc)
{
    if (App::VarSet* existing = find(doc)) {
        return *existing;
    }

    auto* registry = static_cast<App::VarSet*>(doc.addObject("App::VarSet", kRegistry));
    if (registry == nullptr || std::string(registry->getNameInDocument()) != kRegistry) {
        throw Error{"registry-failed",
                    std::string("document ") + doc.getName() + " already holds a " + kRegistry +
                        " that is not a parameter set"};
    }
    return *registry;
}

void require_name(const std::string& name)
{
    if (name.empty()) {
        throw Error{"missing-param", "a parameter name is required"};
    }

    const bool head = std::isalpha(static_cast<unsigned char>(name.front())) != 0 ||
                      name.front() == '_';
    if (!head) {
        throw Error{"invalid-name",
                    name + " must start with a letter or underscore: a slot takes a number or a "
                           "parameter name and nothing may read as both"};
    }
    for (const char letter : name) {
        if (std::isalnum(static_cast<unsigned char>(letter)) == 0 && letter != '_') {
            throw Error{"invalid-name",
                        name + " may only hold letters, digits and underscores"};
        }
    }
}

std::vector<std::string> names(App::Document& doc)
{
    App::VarSet* registry = find(doc);
    if (registry == nullptr) {
        return {};
    }

    std::vector<std::string> out;
    for (const std::string& name : registry->getDynamicPropertyNames()) {
        if (property_of(*registry, name) != nullptr) {
            out.push_back(name);
        }
    }
    return out;
}

bool exists(App::Document& doc, const std::string& name)
{
    App::VarSet* registry = find(doc);
    return registry != nullptr && property_of(*registry, name) != nullptr;
}

double value_of(App::Document& doc, const std::string& name)
{
    return require_property(doc, name).getValue();
}

double resolve(App::Document& doc, const Slot& slot)
{
    return slot.bound() ? value_of(doc, slot.parameter) : slot.value;
}

void apply(App::Document& doc,
           App::DocumentObject& object,
           const std::string& path,
           const Slot& slot)
{
    if (slot.bound()) {
        // Refuses with the same message every other read of a missing
        // parameter gives, rather than a second wording for the same mistake.
        (void)value_of(doc, slot.parameter);
    }

    const App::ObjectIdentifier target = identifier(object, path);
    set_expression(object, target,
                   slot.bound() ? std::string(kRegistry) + "." + slot.parameter : std::string());
}

void declare(App::VarSet& registry, const std::string& name, double value)
{
    App::PropertyFloat* property = property_of(registry, name);
    if (property == nullptr) {
        try {
            property = dynamic_cast<App::PropertyFloat*>(
                registry.addDynamicProperty(kType, name.c_str(), "Parameters"));
        }
        catch (const Base::Exception& error) {
            throw Error{"invalid-name", error.what()};
        }
        if (property == nullptr) {
            throw Error{"invalid-name", name + " cannot be a parameter in this document"};
        }
    }

    // A literal overwrites whatever expression used to compute the slot: `param
    // set x 40` on an expression-driven parameter is how you stop it following.
    set_expression(registry, identifier(registry, name), std::string());
    property->setValue(value);
}

void express(App::VarSet& registry, const std::string& name, const std::string& expression)
{
    if (property_of(registry, name) == nullptr) {
        try {
            registry.addDynamicProperty(kType, name.c_str(), "Parameters");
        }
        catch (const Base::Exception& error) {
            throw Error{"invalid-name", error.what()};
        }
    }
    set_expression(registry, identifier(registry, name), expression);
}

Binding binding_of(const App::DocumentObject& object, const std::string& path)
{
    Binding out;

    auto& mutable_object = const_cast<App::DocumentObject&>(object);
    const App::ObjectIdentifier target = identifier(mutable_object, path);
    const auto info = object.getExpression(target);
    if (!info.expression) {
        return out;
    }

    out.expression = info.expression->toString();

    const std::string prefix = std::string(kRegistry) + ".";
    if (out.expression.rfind(prefix, 0) == 0) {
        const std::string tail = out.expression.substr(prefix.size());
        // Only a bare reference is a binding. Anything else is an expression a
        // human put on the geometry by hand, and reporting it as a parameter
        // would hide the arithmetic this design keeps in one place.
        if (tail.find_first_of(" .+-*/()<>=,") == std::string::npos) {
            out.parameter = tail;
        }
    }
    return out;
}

json::Value slot_json(const App::DocumentObject& object, const std::string& path, double value)
{
    const Binding binding = binding_of(object, path);

    json::Value out = json::Value::object();
    out.set("value", json::Value::number(value));
    out.set("parameter", binding.parameter.empty() ? json::Value()
                                                   : json::Value::string(binding.parameter));
    if (!binding.expression.empty() && binding.parameter.empty()) {
        out.set("expression", json::Value::string(binding.expression));
    }
    return out;
}

json::Value drives(App::Document& doc, const std::string& name)
{
    json::Value out = json::Value::array();
    App::VarSet* registry = find(doc);
    if (registry == nullptr) {
        return out;
    }

    for (App::DocumentObject* object : doc.getObjects()) {
        for (const auto& [path, expression] : object->ExpressionEngine.getExpressions()) {
            if (expression == nullptr) {
                continue;
            }
            bool hit = false;
            for (const auto& [identifier, ignored] : expression->getIdentifiers()) {
                (void)ignored;
                if (refers_to(identifier, *registry, name)) {
                    hit = true;
                    break;
                }
            }
            if (!hit) {
                continue;
            }

            json::Value entry = json::Value::object();
            entry.set("object", json::Value::string(object->getNameInDocument()));
            entry.set("slot", json::Value::string(leading_dot_stripped(path.toString())));
            out.push(std::move(entry));
        }
    }
    return out;
}

const char* state_name(State state)
{
    switch (state) {
        case State::Invalid:
            return "invalid";
        case State::NotEvaluated:
            return "not-evaluated";
        case State::Ok:
            break;
    }
    return "ok";
}

Evaluation evaluate(App::VarSet& registry, const std::string& name)
{
    Evaluation out;

    const App::ObjectIdentifier path = identifier(registry, name);
    const auto info = registry.getExpression(path);
    if (!info.expression) {
        return out;
    }

    App::any produced;
    // Three catches because Base::Exception derives from Base::BaseClass and
    // not from std::exception, so one of them cannot stand in for the others.
    // `execute` catches exactly these three, and anything it survives this has
    // to survive too: a listing that raises is useless precisely when the
    // registry is broken, which is the only time anyone reads it closely.
    try {
        produced = info.expression->getValueAsAny();
    }
    catch (const Base::Exception& error) {
        out.state = State::Invalid;
        out.error = error.what();
        return out;
    }
    catch (const std::bad_cast&) {
        out.state = State::Invalid;
        out.error = "the expression produces a value this parameter cannot hold";
        return out;
    }
    catch (const std::exception& error) {
        out.state = State::Invalid;
        out.error = error.what();
        return out;
    }

    App::Property* property = path.getProperty();
    if (property == nullptr) {
        out.state = State::Invalid;
        out.error = "no property " + name + " to hold the result";
        return out;
    }

    try {
        // FreeCAD's own comparison, the one `execute` uses to decide a row need
        // not be written. Comparing doubles here instead would make the answer
        // depend on a rounding judgement that the engine does not make.
        if (!App::isAnyEqual(produced, property->getPathValue(path))) {
            out.state = State::NotEvaluated;
        }
    }
    catch (const Base::Exception&) {
        out.state = State::NotEvaluated;
    }
    catch (const std::exception&) {
        out.state = State::NotEvaluated;
    }
    return out;
}

Restore capture(App::VarSet& registry, const std::string& name)
{
    Restore out;

    App::PropertyFloat* property = property_of(registry, name);
    if (property == nullptr) {
        return out;
    }

    out.existed = true;
    out.value = property->getValue();

    const auto info = registry.getExpression(identifier(registry, name));
    if (info.expression) {
        out.expression = info.expression->toString();
    }
    return out;
}

void restore(App::VarSet& registry, const std::string& name, const Restore& saved)
{
    const App::ObjectIdentifier path = identifier(registry, name);

    // Cleared first either way. Removing a property the engine still holds an
    // expression for would leave the binding pointing at nothing, which is a
    // worse document than the one being rolled back.
    registry.setExpression(path, std::shared_ptr<App::Expression>());

    if (!saved.existed) {
        registry.removeDynamicProperty(name.c_str());
        return;
    }

    if (!saved.expression.empty()) {
        set_expression(registry, path, saved.expression);
    }
    if (App::PropertyFloat* property = property_of(registry, name)) {
        property->setValue(saved.value);
    }
}

std::vector<std::string> following(App::Document& doc, const App::DocumentObject& object)
{
    std::vector<std::string> out;
    App::VarSet* registry = find(doc);
    if (registry == nullptr || registry == &object) {
        return out;
    }

    for (const auto& [path, expression] : registry->ExpressionEngine.getExpressions()) {
        if (expression == nullptr) {
            continue;
        }
        for (const auto& [identifier, ignored] : expression->getIdentifiers()) {
            (void)ignored;
            // Resolved rather than compared by spelling: an expression may name
            // an object by its label, and the label is not the name.
            if (identifier.getDocumentObject() != &object) {
                continue;
            }
            out.push_back(leading_dot_stripped(path.toString()));
            break;
        }
    }
    return out;
}

}  // namespace ee::params
