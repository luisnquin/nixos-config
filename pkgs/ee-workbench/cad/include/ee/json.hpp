#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace ee::json {

/// Objects keep insertion order, so a reply is byte-stable run to run and a
/// test can compare whole lines instead of reparsing them.
class Value
{
public:
    enum class Kind
    {
        Null,
        Bool,
        Integer,
        Double,
        String,
        Array,
        Object
    };

    Value() = default;

    static Value boolean(bool value);
    static Value integer(long long value);
    static Value number(double value);
    static Value string(std::string value);
    static Value array();
    static Value object();

    Kind kind() const
    {
        return kind_;
    }

    bool is_null() const
    {
        return kind_ == Kind::Null;
    }

    /// Integer only: a client that sends 1.0 or "1" where the protocol says 1
    /// gets a clean refusal instead of a silent coercion.
    std::optional<long long> as_integer() const;
    std::optional<double> as_double() const;
    std::optional<bool> as_bool() const;
    const std::string* as_string() const;

    const std::vector<Value>* as_array() const;

    const Value* find(std::string_view key) const;
    void set(std::string key, Value value);
    void push(Value value);

    std::string dump() const;

private:
    Kind kind_ = Kind::Null;
    bool bool_ = false;
    long long integer_ = 0;
    double double_ = 0.0;
    std::string string_;
    std::vector<Value> items_;
    std::vector<std::pair<std::string, Value>> fields_;
};

/// Strict: no trailing bytes, no comments, no NaN, no unescaped control
/// characters. Returns nothing and fills `error` on anything else.
std::optional<Value> parse(std::string_view text, std::string& error);

}  // namespace ee::json
