{
  coreutils,
  lib,
  runCommand,
  rustPlatform,
}: let
  package = rustPlatform.buildRustPackage {
    pname = "waytools";
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
      substituteInPlace src/main.rs \
        --replace-fail '@who@' '${lib.getExe' coreutils "who"}'
    '';

    meta = {
      description = "Native status helpers for Waybar";
      license = lib.licenses.mit;
      mainProgram = "waytools";
    };
  };

  mkTool = name:
    runCommand name {
      meta.mainProgram = name;
    } ''
      mkdir -p $out/bin
      ln -s ${package}/bin/waytools $out/bin/${name}
    '';
in
  package.overrideAttrs (old: {
    passthru =
      (old.passthru or {})
      // {
        battery = mkTool "waybar-battery";
        ssh = mkTool "waybar-ssh";
        tailscale = mkTool "waybar-tailscale";
      };
  })
