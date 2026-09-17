{
  coreutils,
  lib,
  runCommand,
  rustPlatform,
}: let
  package = rustPlatform.buildRustPackage {
    pname = "barfeed";
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
      mainProgram = "barfeed";
    };
  };

  mkTool = name:
    runCommand name {
      meta.mainProgram = name;
    } ''
      mkdir -p $out/bin
      ln -s ${package}/bin/barfeed $out/bin/${name}
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
