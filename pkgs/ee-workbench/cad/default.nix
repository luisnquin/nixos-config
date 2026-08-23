# Two halves of one FreeCAD session, built from the same sources. The
# standalone server initializes its own headless FreeCAD; the EEWorkbench
# module is loaded by a running FreeCAD GUI through `-M $out/Mod/EEWorkbench`
# and serves the same protocol from the interface's event loop. FreeCAD ships
# no dev output, so the headers come from its source and the generated
# QtCore.h is reconstructed in CMakeLists.txt.
{
  lib,
  stdenv,
  cmake,
  freecad,
  makeWrapper,
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
    "${qt6.qtbase.out}/include/QtGui"
    "${qt6.qtbase.out}/include/QtWidgets"
    "${lib.getDev boost}/include"
    "${microsoft-gsl}/include"
    "${eigen}/include/eigen3"
    "${lib.getDev fmt}/include"
    "${xercesc}/include"
    "${lib.getDev zlib}/include"
  ];

  extraLibraries =
    [
      "${xercesc}/lib/libxerces-c.so"
      "${qt6.qtbase.out}/lib/libQt6Core.so"
      "${python3}/lib/libpython${python3.pythonVersion}.so"
      # PNG's IDAT is a raw zlib stream, so the renderer deflates it itself
      # rather than pulling in an image library for one chunk.
      "${zlib}/lib/libz.so"
    ]
    # The preview mesh is tessellated and written here rather than through a
    # FreeCAD exporter, so the deviations stay a parameter of the request.
    ++ map (name: "${opencascade-occt}/lib/lib${name}.so") [
      "TKernel"
      "TKMath"
      "TKBRep"
      "TKG3d"
      "TKMesh"
      "TKTopAlgo"
    ];

  guiLibraries = [
    "${freecad}/lib/libFreeCADGui.so"
    "${qt6.qtbase.out}/lib/libQt6Gui.so"
  ];

  runtimeLibDirs = [
    "${freecad}/lib"
    "${opencascade-occt}/lib"
    "${xercesc}/lib"
    "${qt6.qtbase.out}/lib"
    "${python3}/lib"
    "${zlib}/lib"
  ];
  serverHome = "libexec/freecad-home/bin";
in
  stdenv.mkDerivation {
    pname = "ee-freecad-server";
    version = "0.1.0";

    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./CMakeLists.txt
        ./Mod
        ./include
        ./src
        ./tests
      ];
    };

    nativeBuildInputs = [cmake makeWrapper];

    cmakeFlags = [
      (lib.cmakeFeature "EE_FREECAD_LIB_DIR" "${freecad}/lib")
      (lib.cmakeFeature "EE_FREECAD_INCLUDE_DIRS" (lib.concatStringsSep ";" includeDirs))
      (lib.cmakeFeature "EE_EXTRA_LIBRARIES" (lib.concatStringsSep ";" extraLibraries))
      (lib.cmakeFeature "EE_GUI_LIBRARIES" (lib.concatStringsSep ";" guiLibraries))
      (lib.cmakeFeature "EE_SERVER_BINDIR" serverHome)
      (lib.cmakeFeature "CMAKE_INSTALL_RPATH" (lib.concatStringsSep ";" runtimeLibDirs))
    ];

    doCheck = true;
    checkTarget = "test";

    # FreeCAD derives its home from /proc/self/exe, so the binary has to sit in
    # a directory shaped like a FreeCAD installation or it finds no modules.
    # That home cannot be $out itself: $out/Mod is where this package keeps the
    # module the GUI loads, not FreeCAD's own.
    postInstall = ''
      for entry in Ext Mod doc lib share; do
        ln -s ${freecad}/$entry $out/${serverHome}/../$entry
      done
      mkdir -p $out/bin
      ln -s ../${serverHome}/ee-freecad-server $out/bin/ee-freecad-server

      # The GUI half is only reachable through FreeCAD's module path, and the
      # module links this exact FreeCAD.
      makeWrapper ${lib.getExe freecad} $out/bin/ee-freecad \
        --add-flags "--module-path $out/Mod/EEWorkbench"
    '';

    meta = {
      description = "Native FreeCAD session server for ee-workbench";
      mainProgram = "ee-freecad-server";
      license = lib.licenses.asl20;
      platforms = lib.platforms.linux;
      sourceProvenance = [lib.sourceTypes.fromSource];
    };
  }
