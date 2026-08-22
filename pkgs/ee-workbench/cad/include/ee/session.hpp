#pragma once

#include <string>

#include "ee/json.hpp"

namespace ee {

/// A refusal the client can branch on: `code` is part of the wire contract,
/// `message` is for a human reading the terminal.
struct Error
{
    std::string code;
    std::string message;
};

/// Every method drives the real FreeCAD document graph in this process.
/// FreeCAD's `App::Application` singleton is the state; this type only holds
/// the rules about which document and which object a request means.
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
    json::Value recompute(const std::string& document);
    json::Value save(const std::string& document, const std::string& path);
    json::Value inspect(const std::string& document) const;
};

}  // namespace ee
