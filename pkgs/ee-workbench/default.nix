# Terminal workbench for personal electronics work: a git repository of
# transparent files, with a CLI and a TUI over it. ./cad holds the native
# FreeCAD server that `ee mechanical` drives over a Unix socket.
{
  lib,
  callPackage,
  rustPlatform,
  makeWrapper,
  git,
}: let
  self = rustPlatform.buildRustPackage {
    pname = "ee-workbench";
    version = "0.1.0";

    # only the crate itself, so editing this file or ./cad does not rebuild it
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
        ./tests
      ];
    };

    cargoLock.lockFile = ./Cargo.lock;

    nativeBuildInputs = [makeWrapper];

    # `ee git` and `ee repo init` shell out; the tests exercise both.
    nativeCheckInputs = [git];

    postInstall = ''
      wrapProgram $out/bin/ee \
        --prefix PATH : ${lib.makeBinPath [git]}
    '';

    passthru = {
      cad = callPackage ./cad {};
      # `nix build .#ee-workbench.tests.slice` runs the CLI against real FreeCAD
      tests.slice = callPackage ./cad/test.nix {ee-workbench = self;};
    };

    meta = {
      description = "Git-backed workbench for projects, inventory, experiments and measurements";
      mainProgram = "ee";
      license = lib.licenses.asl20;
      platforms = lib.platforms.unix;
      sourceProvenance = [lib.sourceTypes.fromSource];
    };
  };
in
  self
