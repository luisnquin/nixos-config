{
  lib,
  mako,
  procps,
  rustPlatform,
}: let
  signal = 9;
in
  rustPlatform.buildRustPackage {
    pname = "pile";
    version = "0.1.0";

    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
      ];
    };

    cargoLock.lockFile = ./Cargo.lock;

    postPatch = ''
      substituteInPlace src/mako.rs \
        --replace-fail '@makoctl@' '${lib.getExe' mako "makoctl"}'
      substituteInPlace src/main.rs \
        --replace-fail '@pkill@' '${lib.getExe' procps "pkill"}' \
        --replace-fail '@signal@' '${toString signal}'
    '';

    passthru = {inherit signal;};

    meta = {
      description = "waybar pile for the notifications mako is told to keep off screen";
      mainProgram = "pile";
      license = lib.licenses.mit;
    };
  }
