{pkgs ? import <nixpkgs> {}}: let
  inherit (pkgs) poetry2nix;
in
  poetry2nix.mkPoetryApplication {
    python = pkgs.python311;

    projectDir = ./.;
    pyproject = ./pyproject.toml;
    poetrylock = ./poetry.lock;
  }
