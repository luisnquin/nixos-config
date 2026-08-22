# A native FreeCAD server: it links the installed FreeCAD, initializes it
# headless and owns a Unix socket that `ee mechanical` talks to. FreeCAD ships
# no dev output, so the headers come from its source and the generated
# QtCore.h is reconstructed in CMakeLists.txt.
{
  lib,
  stdenv,
  cmake,
  freecad,
  opencascade-occt,
  python3,
  qt6,
  boost,
  microsoft-gsl,
  eigen,
  fmt,
  xercesc,
  zlib,
}: let
  includeDirs = [
    "${freecad.src}/src"
    "${freecad.src}/src/3rdParty/PyCXX"
    "${opencascade-occt}/include/opencascade"
    "${python3}/include/python${python3.pythonVersion}"
    # only the dev output's mkspecs are split out; the Qt headers live in `out`
    "${qt6.qtbase.out}/include"
    "${qt6.qtbase.out}/include/QtCore"
    "${lib.getDev boost}/include"
    "${microsoft-gsl}/include"
    "${eigen}/include/eigen3"
    "${lib.getDev fmt}/include"
    "${xercesc}/include"
    "${lib.getDev zlib}/include"
  ];

  extraLibraries = [
    "${xercesc}/lib/libxerces-c.so"
    "${qt6.qtbase.out}/lib/libQt6Core.so"
    "${python3}/lib/libpython${python3.pythonVersion}.so"
  ];

  runtimeLibDirs = [
    "${freecad}/lib"
    "${xercesc}/lib"
    "${qt6.qtbase.out}/lib"
    "${python3}/lib"
  ];
in
  stdenv.mkDerivation {
    pname = "ee-freecad-server";
    version = "0.1.0";

    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./CMakeLists.txt
        ./include
        ./src
        ./tests
      ];
    };

    nativeBuildInputs = [cmake];

    cmakeFlags = [
      (lib.cmakeFeature "EE_FREECAD_LIB_DIR" "${freecad}/lib")
      (lib.cmakeFeature "EE_FREECAD_INCLUDE_DIRS" (lib.concatStringsSep ";" includeDirs))
      (lib.cmakeFeature "EE_EXTRA_LIBRARIES" (lib.concatStringsSep ";" extraLibraries))
      (lib.cmakeFeature "CMAKE_INSTALL_RPATH" (lib.concatStringsSep ";" runtimeLibDirs))
    ];

    doCheck = true;
    checkTarget = "test";

    # FreeCAD derives its home from /proc/self/exe, so the binary has to sit in
    # a directory shaped like a FreeCAD installation or it finds no modules.
    postInstall = ''
      for entry in Ext Mod doc lib share; do
        ln -s ${freecad}/$entry $out/$entry
      done
    '';

    meta = {
      description = "Native FreeCAD session server for ee-workbench";
      mainProgram = "ee-freecad-server";
      license = lib.licenses.asl20;
      platforms = lib.platforms.linux;
      sourceProvenance = [lib.sourceTypes.fromSource];
    };
  }
