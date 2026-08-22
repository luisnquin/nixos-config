cmake_minimum_required(VERSION 3.20)

project(ee-freecad-server LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(EE_FREECAD_LIB_DIR "" CACHE PATH "Directory holding libFreeCADApp.so and friends")
set(EE_FREECAD_INCLUDE_DIRS "" CACHE STRING "Include directories for FreeCAD and its dependencies")
set(EE_EXTRA_LIBRARIES "" CACHE STRING "Absolute paths of the non-FreeCAD libraries to link")
set(EE_GUI_LIBRARIES "" CACHE STRING "Absolute paths of the libraries only the GUI module needs")
set(EE_SERVER_BINDIR "bin" CACHE STRING "Where the standalone server lands, relative to the prefix")
option(EE_BUILD_TESTS "Build the JSON self-test" ON)

# Strict JSON with no third-party dependency: the wire format is small, and the
# protocol guards depend on integers staying integers.
add_library(ee_json STATIC src/json.cpp)
target_include_directories(ee_json PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_compile_options(ee_json PRIVATE -Wall -Wextra -Wpedantic)

if(EE_BUILD_TESTS)
  enable_testing()
  add_executable(ee-json-test tests/json_test.cpp)
  target_link_libraries(ee-json-test PRIVATE ee_json)
  target_compile_options(ee-json-test PRIVATE -Wall -Wextra -Wpedantic)
  add_test(NAME json COMMAND ee-json-test)
endif()

if(NOT EE_FREECAD_LIB_DIR)
  message(STATUS "EE_FREECAD_LIB_DIR is unset; building the JSON library only")
  return()
endif()

# FreeCAD generates this header from src/QtCore.h.cmake and ships no dev output,
# so an out-of-tree build has to reconstruct it.
file(WRITE ${CMAKE_CURRENT_BINARY_DIR}/gen/QtCore.h
  "#ifndef FREECAD_QTCORE_H\n#define FREECAD_QTCORE_H\n#define HAVE_Q_DISABLE_COPY_MOVE\n#endif\n")

# FreeCAD's workbench libraries are Python extension modules: Part.so, not
# libPart.so, so the empty prefix has to be allowed.
set(CMAKE_FIND_LIBRARY_PREFIXES "lib" "")

foreach(lib FreeCADApp FreeCADBase Part Sketcher _PartDesign)
  find_library(EE_LIB_${lib} NAMES ${lib} PATHS ${EE_FREECAD_LIB_DIR} NO_DEFAULT_PATH)
  if(NOT EE_LIB_${lib})
    message(FATAL_ERROR "missing ${lib} in ${EE_FREECAD_LIB_DIR}")
  endif()
  list(APPEND EE_FREECAD_LIBS ${EE_LIB_${lib}})
endforeach()

# Everything both halves share: the document operations, the request
# dispatcher and the mesh export. Only the transport differs.
add_library(ee_session STATIC
  src/listener.cpp
  src/mesh.cpp
  src/paths.cpp
  src/protocol.cpp
  src/session.cpp)
target_include_directories(ee_session PUBLIC
  ${CMAKE_CURRENT_BINARY_DIR}/gen
  ${EE_FREECAD_INCLUDE_DIRS})
target_link_libraries(ee_session PUBLIC ee_json ${EE_FREECAD_LIBS} ${EE_EXTRA_LIBRARIES})
# Qt's `slots` keyword macro collides with CPython's PyType_Slot member.
target_compile_definitions(ee_session PUBLIC QT_NO_KEYWORDS)
target_compile_options(ee_session PRIVATE -Wall -Wextra)
set_property(TARGET ee_session PROPERTY POSITION_INDEPENDENT_CODE ON)

# The standalone session: its own FreeCAD, no GUI, one blocking accept loop.
add_executable(ee-freecad-server src/main.cpp src/server.cpp src/gui_headless.cpp)
target_link_libraries(ee-freecad-server PRIVATE ee_session)
target_compile_options(ee-freecad-server PRIVATE -Wall -Wextra)

# The GUI half: a Python extension module FreeCAD loads from its module path,
# serving the same protocol from the interface's own event loop.
add_library(EEWorkbench MODULE src/module.cpp src/qtserver.cpp src/gui_loaded.cpp)
set_target_properties(EEWorkbench PROPERTIES PREFIX "" SUFFIX ".so")
target_link_libraries(EEWorkbench PRIVATE ee_session ${EE_GUI_LIBRARIES})
target_compile_options(EEWorkbench PRIVATE -Wall -Wextra)

include(GNUInstallDirs)
install(TARGETS ee-freecad-server RUNTIME DESTINATION ${EE_SERVER_BINDIR})
install(TARGETS EEWorkbench LIBRARY DESTINATION Mod/EEWorkbench)
install(FILES Mod/EEWorkbench/InitGui.py DESTINATION Mod/EEWorkbench)
