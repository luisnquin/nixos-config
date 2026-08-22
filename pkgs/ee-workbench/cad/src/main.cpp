#include <clocale>
#include <csignal>
#include <cstdlib>
#include <iostream>
#include <string>
#include <system_error>

#include <pwd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#include <App/Application.h>
#include <Base/Console.h>
#include <Base/Interpreter.h>

#include "ee/json.hpp"
#include "ee/server.hpp"

namespace {

void on_signal(int)
{
    ee::g_interrupted = 1;
}

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

/// Mirrors `paths::cad_socket` on the Rust side; the two must agree or the
/// client never finds the server.
std::string default_socket_path()
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

void ensure_parent_directory(const std::string& path)
{
    const std::size_t slash = path.rfind('/');
    if (slash == std::string::npos || slash == 0) {
        return;
    }
    const std::string parent = path.substr(0, slash);
    if (::mkdir(parent.c_str(), 0700) != 0 && errno != EEXIST) {
        throw std::system_error(errno, std::generic_category(), "mkdir " + parent);
    }
}

void usage(std::ostream& out)
{
    out << "ee-freecad-server [--socket PATH]\n\n"
        << "Owns a FreeCAD document session and serves NDJSON requests on a\n"
        << "Unix socket. `ee mechanical` is the client.\n";
}

}  // namespace

int main(int argc, char** argv)
{
    std::string socket_path;

    for (int i = 1; i < argc; ++i) {
        const std::string flag = argv[i];
        if (flag == "--help" || flag == "-h") {
            usage(std::cout);
            return 0;
        }
        if (flag == "--socket") {
            if (i + 1 >= argc) {
                std::cerr << "--socket needs a path\n";
                return 2;
            }
            socket_path = argv[++i];
            continue;
        }
        if (flag.rfind("--socket=", 0) == 0) {
            socket_path = flag.substr(std::string("--socket=").size());
            continue;
        }
        std::cerr << "unknown argument: " << flag << "\n";
        usage(std::cerr);
        return 2;
    }

    if (socket_path.empty()) {
        socket_path = default_socket_path();
    }

    std::setlocale(LC_ALL, "");
    // FreeCAD writes and parses numbers with the C locale; a comma decimal
    // separator silently corrupts saved documents.
    std::setlocale(LC_NUMERIC, "C");

    App::Application::Config()["ExeName"] = "ee-freecad-server";
    App::Application::Config()["ExeVendor"] = "ee-workbench";
    App::Application::Config()["AppDataSkipVendor"] = "true";
    App::Application::Config()["RunMode"] = "Exit";
    App::Application::Config()["LoggingConsole"] = "0";

    try {
        ensure_parent_directory(socket_path);

        // FreeCAD parses the command line itself and rejects flags it does not
        // know, so it never sees ours.
        char program[] = "ee-freecad-server";
        char* freecad_argv[] = {program, nullptr};
        App::Application::init(1, freecad_argv);

        {
            Base::PyGILStateLocker lock;
            Base::Interpreter().loadModule("Part");
            Base::Interpreter().loadModule("Sketcher");
            Base::Interpreter().loadModule("PartDesign");
        }

        struct sigaction action{};
        action.sa_handler = on_signal;
        ::sigaction(SIGINT, &action, nullptr);
        ::sigaction(SIGTERM, &action, nullptr);
        ::signal(SIGPIPE, SIG_IGN);

        ee::Server server(socket_path);
        server.listen();

        ee::json::Value ready = ee::json::Value::object();
        ready.set("ready", ee::json::Value::boolean(true));
        ready.set("protocol", ee::json::Value::integer(ee::kProtocol));
        ready.set("socket", ee::json::Value::string(server.socket_path()));
        std::cout << ready.dump() << std::endl;

        server.run();
    }
    catch (const Base::Exception& error) {
        std::cerr << "freecad: " << error.what() << "\n";
        return 1;
    }
    catch (const std::exception& error) {
        std::cerr << "ee-freecad-server: " << error.what() << "\n";
        return 1;
    }

    return 0;
}
