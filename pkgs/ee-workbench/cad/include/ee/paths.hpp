#pragma once

#include <string>

namespace ee::paths {

/// Mirrors `paths::cad_socket` on the Rust side; the two must agree or the
/// client never finds the session.
std::string default_socket();

/// `$XDG_CACHE_HOME/ee-workbench/preview`, the stable home of the meshes a
/// viewer keeps open. Derived state only: losing it costs a re-export.
std::string preview_dir();

/// mkdir -p with 0700, for a file path's parent.
void ensure_parent(const std::string& path);

}  // namespace ee::paths
