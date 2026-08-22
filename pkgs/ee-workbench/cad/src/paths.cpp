#include "ee/paths.hpp"

#include <cerrno>
#include <cstdlib>
#include <string>
#include <system_error>

#include <pwd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

namespace ee::paths {
namespace {

std::string env_or_empty(const char* name)
{
    const char* value = std::getenv(name);
    return value != nullptr ? std::string(value) : std::string();
}

/// Same order as the Rust side: the environment first, so both binaries derive
/// the same path even when they disagree with the password database.
std::string user_name()
{
    for (const char* variable : {"USER", "LOGNAME"}) {
        if (const std::string value = env_or_empty(variable); !value.empty()) {
            return value;
        }
    }
    if (const passwd* entry = ::getpwuid(::getuid())) {
        if (entry->pw_name != nullptr) {
            return entry->pw_name;
        }
    }
    return "nobody";
}

std::string home()
{
    const std::string value = env_or_empty("HOME");
    return value.empty() ? std::string(".") : value;
}

std::string xdg(const char* variable, const char* fallback)
{
    const std::string value = env_or_empty(variable);
    return value.empty() ? home() + "/" + fallback : value;
}

}  // namespace

std::string default_socket()
{
    if (const std::string explicit_path = env_or_empty("EE_WORKBENCH_CAD_SOCKET");
        !explicit_path.empty()) {
        return explicit_path;
    }

    std::string base = env_or_empty("XDG_RUNTIME_DIR");
    if (!base.empty()) {
        base += "/ee-workbench";
    }
    else {
        std::string tmp = env_or_empty("TMPDIR");
        if (tmp.empty()) {
            tmp = "/tmp";
        }
        base = tmp + "/ee-workbench-" + user_name();
    }
    return base + "/cad.sock";
}

std::string preview_dir()
{
    return xdg("XDG_CACHE_HOME", ".cache") + "/ee-workbench/preview";
}

void ensure_parent(const std::string& path)
{
    const std::size_t slash = path.rfind('/');
    if (slash == std::string::npos || slash == 0) {
        return;
    }

    const std::string parent = path.substr(0, slash);
    for (std::size_t at = parent.find('/', 1); ; at = parent.find('/', at + 1)) {
        const std::string step = at == std::string::npos ? parent : parent.substr(0, at);
        if (::mkdir(step.c_str(), 0700) != 0 && errno != EEXIST) {
            throw std::system_error(errno, std::generic_category(), "mkdir " + step);
        }
        if (at == std::string::npos) {
            break;
        }
    }
}

}  // namespace ee::paths
