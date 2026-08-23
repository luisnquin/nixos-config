# Terminal workbench for personal electronics work: a git repository of
# transparent files, with a CLI and a TUI over it. ./cad holds the native
# FreeCAD server that `ee mechanical` drives over a Unix socket.
{
  lib,
  callPackage,
  rustPlatform,
  makeWrapper,
  runCommandLocal,
  git,
}: let
  version = "0.1.0";

  meta = {
    description = "Git-backed workbench for projects, inventory, experiments and measurements";
    mainProgram = "ee";
    license = lib.licenses.asl20;
    platforms = lib.platforms.unix;
    sourceProvenance = [lib.sourceTypes.fromSource];
  };

  cad = callPackage ./cad {};

  self = rustPlatform.buildRustPackage {
    pname = "ee-workbench";
    inherit version;

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
      inherit cad withCad;
      # `nix build .#ee-workbench.tests.slice` runs the CLI against real FreeCAD
      tests.slice = callPackage ./cad/test.nix {ee-workbench = withCad;};
    };

    inherit meta;
  };

  # `ee` and the server it was built against, as one closure. Naming the server
  # by store path is what makes it a runtime reference: install this and the two
  # cannot drift apart, where a session variable would keep pointing at whatever
  # generation the shell was born in. `--set` and not `--set-default` for the
  # same reason — a stale value already in the environment has to lose.
  withCad =
    runCommandLocal "ee-workbench-cad-${version}" {
      nativeBuildInputs = [makeWrapper];
      passthru = {
        inherit cad;
        client = self;
      };
      inherit meta;
    } ''
      mkdir -p $out/bin
      makeWrapper ${lib.getExe self} $out/bin/ee \
        --set EE_WORKBENCH_CAD_SERVER ${lib.getExe cad} \
        --set EE_WORKBENCH_CAD_BUILD ${cad}
    '';
in
  self
