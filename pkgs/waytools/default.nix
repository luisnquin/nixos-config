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
        sshSolo = mkTool "waybar-ssh-solo";
        sshIn = mkTool "waybar-ssh-in";
        sshOut = mkTool "waybar-ssh-out";
        tailscale = mkTool "waybar-tailscale";
      };
  })
