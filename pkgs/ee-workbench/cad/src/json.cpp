#include "ee/json.hpp"

#include <array>
#include <charconv>
#include <cmath>
#include <cstdint>
#include <system_error>

namespace ee::json {
namespace {

bool is_ws(char c)
{
    return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

void encode_utf8(std::uint32_t code, std::string& out)
{
    if (code < 0x80) {
        out.push_back(static_cast<char>(code));
    }
    else if (code < 0x800) {
        out.push_back(static_cast<char>(0xC0 | (code >> 6)));
        out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
    }
    else if (code < 0x10000) {
        out.push_back(static_cast<char>(0xE0 | (code >> 12)));
        out.push_back(static_cast<char>(0x80 | ((code >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
    }
    else {
        out.push_back(static_cast<char>(0xF0 | (code >> 18)));
        out.push_back(static_cast<char>(0x80 | ((code >> 12) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | ((code >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
    }
}

class Parser
{
public:
    Parser(std::string_view text, std::string& error)
        : text_(text)
        , error_(error)
    {}

    std::optional<Value> run()
    {
        skip_ws();

        auto value = parse_value(0);
        if (!value) {
            return std::nullopt;
        }

        skip_ws();

        if (at_ < text_.size()) {
            return fail("trailing bytes after the JSON value");
        }

        return value;
    }

private:
    static constexpr int kMaxDepth = 32;

    std::optional<Value> fail(const std::string& what)
    {
        error_ = what + " at byte " + std::to_string(at_);

        return std::nullopt;
    }

    void skip_ws()
    {
        while (at_ < text_.size() && is_ws(text_[at_])) {
            ++at_;
        }
    }

    bool literal(std::string_view word)
    {
        if (text_.compare(at_, word.size(), word) != 0) {
            return false;
        }

        at_ += word.size();

        return true;
    }

    std::optional<Value> parse_value(int depth)
    {
        if (depth > kMaxDepth) {
            return fail("nesting is deeper than the protocol allows");
        }

        if (at_ >= text_.size()) {
            return fail("the value is empty");
        }

        switch (text_[at_]) {
            case '{':
                return parse_object(depth);
            case '[':
                return parse_array(depth);
            case '"': {
                std::string out;
                if (!parse_string(out)) {
                    return std::nullopt;
                }
                return Value::string(std::move(out));
            }
            case 't':
                return literal("true") ? std::optional(Value::boolean(true))
                                       : fail("expected true");
            case 'f':
                return literal("false") ? std::optional(Value::boolean(false))
                                        : fail("expected false");
            case 'n':
                return literal("null") ? std::optional(Value()) : fail("expected null");
            default:
                return parse_number();
        }
    }

    std::optional<Value> parse_object(int depth)
    {
        ++at_;
        skip_ws();

        Value out = Value::object();

        if (at_ < text_.size() && text_[at_] == '}') {
            ++at_;
            return out;
        }

        while (true) {
            skip_ws();

            if (at_ >= text_.size() || text_[at_] != '"') {
                return fail("expected a quoted member name");
            }

            std::string key;
            if (!parse_string(key)) {
                return std::nullopt;
            }

            skip_ws();

            if (at_ >= text_.size() || text_[at_] != ':') {
                return fail("expected ':' after a member name");
            }

            ++at_;
            skip_ws();

            auto value = parse_value(depth + 1);
            if (!value) {
                return std::nullopt;
            }

            out.set(std::move(key), std::move(*value));

            skip_ws();

            if (at_ >= text_.size()) {
                return fail("the object is not closed");
            }

            if (text_[at_] == ',') {
                ++at_;
                continue;
            }

            if (text_[at_] == '}') {
                ++at_;
                return out;
            }

            return fail("expected ',' or '}'");
        }
    }

    std::optional<Value> parse_array(int depth)
    {
        ++at_;
        skip_ws();

        Value out = Value::array();

        if (at_ < text_.size() && text_[at_] == ']') {
            ++at_;
            return out;
        }

        while (true) {
            skip_ws();

            auto value = parse_value(depth + 1);
            if (!value) {
                return std::nullopt;
            }

            out.push(std::move(*value));

            skip_ws();

            if (at_ >= text_.size()) {
                return fail("the array is not closed");
            }

            if (text_[at_] == ',') {
                ++at_;
                continue;
            }

            if (text_[at_] == ']') {
                ++at_;
                return out;
            }

            return fail("expected ',' or ']'");
        }
    }

    bool parse_string(std::string& out)
    {
        ++at_;

        while (true) {
            if (at_ >= text_.size()) {
                fail("the string is not closed");
                return false;
            }

            const auto c = static_cast<unsigned char>(text_[at_]);

            if (c == '"') {
                ++at_;
                return true;
            }

            if (c < 0x20) {
                fail("a raw control character is not valid inside a string");
                return false;
            }

            if (c != '\\') {
                out.push_back(text_[at_++]);
                continue;
            }

            ++at_;

            if (at_ >= text_.size()) {
                fail("the escape is not finished");
                return false;
            }

            switch (text_[at_++]) {
                case '"':
                    out.push_back('"');
                    break;
                case '\\':
                    out.push_back('\\');
                    break;
                case '/':
                    out.push_back('/');
                    break;
                case 'b':
                    out.push_back('\b');
                    break;
                case 'f':
                    out.push_back('\f');
                    break;
                case 'n':
                    out.push_back('\n');
                    break;
                case 'r':
                    out.push_back('\r');
                    break;
                case 't':
                    out.push_back('\t');
                    break;
                case 'u': {
                    std::uint32_t code = 0;
                    if (!parse_hex4(code)) {
                        return false;
                    }

                    // A high surrogate is only meaningful paired with its low
                    // half; anything else would encode an invalid code point.
                    if (code >= 0xD800 && code <= 0xDBFF) {
                        if (at_ + 1 >= text_.size() || text_[at_] != '\\'
                            || text_[at_ + 1] != 'u') {
                            fail("a high surrogate is missing its pair");
                            return false;
                        }

                        at_ += 2;

                        std::uint32_t low = 0;
                        if (!parse_hex4(low)) {
                            return false;
                        }

                        if (low < 0xDC00 || low > 0xDFFF) {
                            fail("a high surrogate is followed by a non-surrogate");
                            return false;
                        }

                        code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                    }
                    else if (code >= 0xDC00 && code <= 0xDFFF) {
                        fail("a low surrogate appears without a high surrogate");
                        return false;
                    }

                    encode_utf8(code, out);
                    break;
                }
                default:
                    fail("unknown escape");
                    return false;
            }
        }
    }

    bool parse_hex4(std::uint32_t& code)
    {
        if (at_ + 4 > text_.size()) {
            fail("a \\u escape needs four hex digits");
            return false;
        }

        code = 0;

        for (int i = 0; i < 4; ++i) {
            const char c = text_[at_ + i];
            code <<= 4;

            if (c >= '0' && c <= '9') {
                code |= static_cast<std::uint32_t>(c - '0');
            }
            else if (c >= 'a' && c <= 'f') {
                code |= static_cast<std::uint32_t>(c - 'a' + 10);
            }
            else if (c >= 'A' && c <= 'F') {
                code |= static_cast<std::uint32_t>(c - 'A' + 10);
            }
            else {
                fail("a \\u escape needs four hex digits");
                return false;
            }
        }

        at_ += 4;

        return true;
    }

    std::optional<Value> parse_number()
    {
        const std::size_t start = at_;

        if (at_ < text_.size() && text_[at_] == '-') {
            ++at_;
        }

        const std::size_t digits = at_;

        while (at_ < text_.size() && text_[at_] >= '0' && text_[at_] <= '9') {
            ++at_;
        }

        if (at_ == digits) {
            return fail("expected a number");
        }

        if (text_[digits] == '0' && at_ - digits > 1) {
            return fail("a number may not have a leading zero");
        }

        bool fractional = false;

        if (at_ < text_.size() && (text_[at_] == '.' || text_[at_] == 'e' || text_[at_] == 'E')) {
            fractional = true;

            if (text_[at_] == '.') {
                ++at_;
                const std::size_t frac = at_;

                while (at_ < text_.size() && text_[at_] >= '0' && text_[at_] <= '9') {
                    ++at_;
                }

                if (at_ == frac) {
                    return fail("the fraction has no digits");
                }
            }

            if (at_ < text_.size() && (text_[at_] == 'e' || text_[at_] == 'E')) {
                ++at_;

                if (at_ < text_.size() && (text_[at_] == '+' || text_[at_] == '-')) {
                    ++at_;
                }

                const std::size_t exp = at_;

                while (at_ < text_.size() && text_[at_] >= '0' && text_[at_] <= '9') {
                    ++at_;
                }

                if (at_ == exp) {
                    return fail("the exponent has no digits");
                }
            }
        }

        const std::string_view token = text_.substr(start, at_ - start);

        if (!fractional) {
            long long integer = 0;
            const auto result =
                std::from_chars(token.data(), token.data() + token.size(), integer);

            if (result.ec == std::errc {} && result.ptr == token.data() + token.size()) {
                return Value::integer(integer);
            }
        }

        double number = 0.0;
        const auto result = std::from_chars(token.data(), token.data() + token.size(), number);

        if (result.ec != std::errc {} || result.ptr != token.data() + token.size()) {
            return fail("the number is out of range");
        }

        return Value::number(number);
    }

    std::string_view text_;
    std::string& error_;
    std::size_t at_ = 0;
};

void dump_string(const std::string& value, std::string& out)
{
    out.push_back('"');

    for (const char raw : value) {
        const auto c = static_cast<unsigned char>(raw);

        switch (c) {
            case '"':
                out += "\\\"";
                break;
            case '\\':
                out += "\\\\";
                break;
            case '\b':
                out += "\\b";
                break;
            case '\f':
                out += "\\f";
                break;
            case '\n':
                out += "\\n";
                break;
            case '\r':
                out += "\\r";
                break;
            case '\t':
                out += "\\t";
                break;
            default:
                if (c < 0x20) {
                    static constexpr std::array<char, 17> hex {"0123456789abcdef"};
                    out += "\\u00";
                    out.push_back(hex[(c >> 4) & 0xF]);
                    out.push_back(hex[c & 0xF]);
                }
                else {
                    out.push_back(raw);
                }
        }
    }

    out.push_back('"');
}

void dump_double(double value, std::string& out)
{
    // JSON has no infinity and no NaN; a value that cannot round-trip is a
    // bug on our side, so say so in-band instead of writing invalid JSON.
    if (!std::isfinite(value)) {
        out += "null";
        return;
    }

    std::array<char, 40> buffer {};
    const auto result = std::to_chars(buffer.data(), buffer.data() + buffer.size(), value);

    out.append(buffer.data(), result.ptr);
}

}  // namespace

Value Value::boolean(bool value)
{
    Value out;
    out.kind_ = Kind::Bool;
    out.bool_ = value;

    return out;
}

Value Value::integer(long long value)
{
    Value out;
    out.kind_ = Kind::Integer;
    out.integer_ = value;

    return out;
}

Value Value::number(double value)
{
    Value out;
    out.kind_ = Kind::Double;
    out.double_ = value;

    return out;
}

Value Value::string(std::string value)
{
    Value out;
    out.kind_ = Kind::String;
    out.string_ = std::move(value);

    return out;
}

Value Value::array()
{
    Value out;
    out.kind_ = Kind::Array;

    return out;
}

Value Value::object()
{
    Value out;
    out.kind_ = Kind::Object;

    return out;
}

std::optional<long long> Value::as_integer() const
{
    if (kind_ != Kind::Integer) {
        return std::nullopt;
    }

    return integer_;
}

std::optional<double> Value::as_double() const
{
    if (kind_ == Kind::Integer) {
        return static_cast<double>(integer_);
    }

    if (kind_ != Kind::Double) {
        return std::nullopt;
    }

    return double_;
}

std::optional<bool> Value::as_bool() const
{
    if (kind_ != Kind::Bool) {
        return std::nullopt;
    }

    return bool_;
}

const std::string* Value::as_string() const
{
    return kind_ == Kind::String ? &string_ : nullptr;
}

const std::vector<Value>* Value::as_array() const
{
    return kind_ == Kind::Array ? &items_ : nullptr;
}

const Value* Value::find(std::string_view key) const
{
    if (kind_ != Kind::Object) {
        return nullptr;
    }

    for (const auto& field : fields_) {
        if (field.first == key) {
            return &field.second;
        }
    }

    return nullptr;
}

void Value::set(std::string key, Value value)
{
    kind_ = Kind::Object;

    for (auto& field : fields_) {
        if (field.first == key) {
            field.second = std::move(value);
            return;
        }
    }

    fields_.emplace_back(std::move(key), std::move(value));
}

void Value::push(Value value)
{
    kind_ = Kind::Array;
    items_.push_back(std::move(value));
}

std::string Value::dump() const
{
    std::string out;

    switch (kind_) {
        case Kind::Null:
            out += "null";
            break;
        case Kind::Bool:
            out += bool_ ? "true" : "false";
            break;
        case Kind::Integer:
            out += std::to_string(integer_);
            break;
        case Kind::Double:
            dump_double(double_, out);
            break;
        case Kind::String:
            dump_string(string_, out);
            break;
        case Kind::Array: {
            out.push_back('[');

            for (std::size_t i = 0; i < items_.size(); ++i) {
                if (i > 0) {
                    out.push_back(',');
                }

                out += items_[i].dump();
            }

            out.push_back(']');
            break;
        }
        case Kind::Object: {
            out.push_back('{');

            for (std::size_t i = 0; i < fields_.size(); ++i) {
                if (i > 0) {
                    out.push_back(',');
                }

                dump_string(fields_[i].first, out);
                out.push_back(':');
                out += fields_[i].second.dump();
            }

            out.push_back('}');
            break;
        }
    }

    return out;
}

std::optional<Value> parse(std::string_view text, std::string& error)
{
    Parser parser(text, error);

    return parser.run();
}

}  // namespace ee::json
