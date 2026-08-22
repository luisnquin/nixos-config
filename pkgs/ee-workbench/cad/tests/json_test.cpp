#include "ee/json.hpp"

#include <iostream>
#include <string>

namespace {

int g_failures = 0;

void check(bool condition, const std::string& what)
{
    if (!condition) {
        std::cerr << "FAIL: " << what << "\n";
        ++g_failures;
    }
}

ee::json::Value must_parse(const std::string& text)
{
    std::string error;
    std::optional<ee::json::Value> value = ee::json::parse(text, error);
    check(value.has_value(), "parse " + text + ": " + error);
    return value.value_or(ee::json::Value());
}

void must_reject(const std::string& text)
{
    std::string error;
    check(!ee::json::parse(text, error).has_value(), "should reject " + text);
}

}  // namespace

int main()
{
    const ee::json::Value request =
        must_parse(R"({"protocol":2,"id":7,"method":"sketch.rectangle",)"
                   R"("params":{"width":40,"height":25.5,"driving":true,"name":null}})");

    const ee::json::Value* protocol = request.find("protocol");
    check(protocol != nullptr && protocol->as_integer() == 2, "protocol is an integer");

    const ee::json::Value* params = request.find("params");
    check(params != nullptr, "params present");
    if (params != nullptr) {
        const ee::json::Value* width = params->find("width");
        check(width != nullptr && width->as_integer() == 40, "40 stays an integer");
        const ee::json::Value* height = params->find("height");
        check(height != nullptr && !height->as_integer().has_value(), "25.5 is not an integer");
        check(height != nullptr && height->as_double() == 25.5, "25.5 reads as a double");
        const ee::json::Value* driving = params->find("driving");
        check(driving != nullptr && driving->as_bool() == true, "booleans survive");
        const ee::json::Value* name = params->find("name");
        check(name != nullptr && name->is_null(), "null is a value, not an absence");
        check(params->find("missing") == nullptr, "absent keys are absent");
    }

    // A float where an integer is required is what the protocol guard must catch.
    const ee::json::Value loose = must_parse(R"({"protocol":2.0})");
    const ee::json::Value* loose_protocol = loose.find("protocol");
    check(loose_protocol != nullptr && !loose_protocol->as_integer().has_value(),
          "2.0 is not accepted as the integer 2");

    const ee::json::Value escaped = must_parse(R"({"text":"tab\tquote\"\u00e9\ud83d\ude00"})");
    const std::string* text = escaped.find("text")->as_string();
    check(text != nullptr && *text == "tab\tquote\"\xc3\xa9\xf0\x9f\x98\x80", "escapes decode");

    must_reject("");
    must_reject("{");
    must_reject("{\"a\":1}trailing");
    must_reject("{\"a\":}");
    must_reject("{\"a\":01}");
    must_reject("{\"a\":1.}");
    must_reject("{\"a\":1e}");
    must_reject("{'a':1}");
    must_reject("{\"a\":\"raw\ncontrol\"}");
    must_reject("{\"a\":\"\\ud800\"}");
    must_reject(std::string(64, '[') + std::string(64, ']'));

    ee::json::Value reply = ee::json::Value::object();
    reply.set("ok", ee::json::Value::boolean(true));
    reply.set("protocol", ee::json::Value::integer(2));
    reply.set("id", ee::json::Value());
    ee::json::Value items = ee::json::Value::array();
    items.push(ee::json::Value::string("a\nb"));
    items.push(ee::json::Value::number(1.5));
    reply.set("result", items);
    check(reply.dump() == R"({"ok":true,"protocol":2,"id":null,"result":["a\nb",1.5]})",
          "dump keeps insertion order: " + reply.dump());

    reply.set("ok", ee::json::Value::boolean(false));
    check(reply.dump().rfind(R"({"ok":false,)", 0) == 0, "set replaces in place");

    if (g_failures == 0) {
        std::cout << "ok\n";
    }
    return g_failures == 0 ? 0 : 1;
}
