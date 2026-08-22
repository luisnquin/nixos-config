#include <clocale>
#include <csignal>
#include <cstdlib>
#include <iostream>
#include <string>

#include <App/Application.h>
#include <Base/Console.h>
#include <Base/Interpreter.h>

#include "ee/json.hpp"
#include "ee/paths.hpp"
#include "ee/server.hpp"

namespace {

void on_signal(int)
{
    ee::g_interrupted = 1;
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
        socket_path = ee::paths::default_socket();
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
        ee::paths::ensure_parent(socket_path);

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
