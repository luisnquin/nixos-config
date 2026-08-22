// The FreeCAD GUI half of the session. FreeCAD discovers modules by executing
// Init.py/InitGui.py in each module directory, so a Python import is the only
// entry point it offers; everything past that line is C++. `Mod/EEWorkbench/
// InitGui.py` is that one line, and exists for no other reason.
#include <Python.h>

#include <exception>
#include <string>

#include <QCoreApplication>

#include <Base/Console.h>
#include <Base/Interpreter.h>

#include "ee/paths.hpp"
#include "ee/protocol.hpp"
#include "ee/qtserver.hpp"

namespace {

ee::QtServer* g_server = nullptr;

void start_server()
{
    if (g_server != nullptr) {
        return;
    }
    if (QCoreApplication::instance() == nullptr) {
        Base::Console().warning(
            "ee-workbench: no Qt application yet, the CAD session was not started\n");
        return;
    }

    try {
        // Linking the workbench libraries is not enough: their C++ types are
        // registered by the module init function, which only runs on import.
        Base::Interpreter().loadModule("Part");
        Base::Interpreter().loadModule("Sketcher");
        Base::Interpreter().loadModule("PartDesign");

        const std::string socket = ee::paths::default_socket();
        ee::paths::ensure_parent(socket);

        auto* server = new ee::QtServer(socket, QCoreApplication::instance());
        server->start();
        g_server = server;

        Base::Console().message("ee-workbench: CAD session on %s (protocol %lld)\n",
                                socket.c_str(),
                                ee::kProtocol);
    }
    catch (const std::exception& error) {
        Base::Console().error("ee-workbench: %s\n", error.what());
    }
}

PyObject* module_socket(PyObject*, PyObject*)
{
    if (g_server == nullptr) {
        Py_RETURN_NONE;
    }
    return PyUnicode_FromString(g_server->socket_path().c_str());
}

PyMethodDef methods[] = {
    {"socket", module_socket, METH_NOARGS, "Path of the socket this session listens on."},
    {nullptr, nullptr, 0, nullptr},
};

PyModuleDef definition = {
    PyModuleDef_HEAD_INIT,
    "EEWorkbench",
    "ee-workbench CAD session, served on the FreeCAD GUI event loop.",
    -1,
    methods,
    nullptr,
    nullptr,
    nullptr,
    nullptr,
};

}  // namespace

PyMODINIT_FUNC PyInit_EEWorkbench()
{
    PyObject* module = PyModule_Create(&definition);
    if (module == nullptr) {
        return nullptr;
    }
    start_server();
    return module;
}
