{
  lib,
  rustPlatform,
}: let
  package = rustPlatform.buildRustPackage {
    pname = "nyx-mcp-gateway";
    version = "0.1.0";

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

    meta = {
      description = "Nix-native shared MCP process gateway";
      mainProgram = "mcp-gw-client";
    };
  };
in
  package.overrideAttrs (old: {
    passthru =
      (old.passthru or {})
      // {
        daemon = "${package}/bin/mcp-gatewayd";
        proxy = "${package}/bin/mcp-gw-client";
      };
  })
