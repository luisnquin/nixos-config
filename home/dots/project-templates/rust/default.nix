{pkgs ? import <nixpkgs> {}, ...}: let
  runtimePackages = with pkgs; [];
in
  pkgs.rustPlatform.buildRustPackage rec {
    pname = "rust-app";
    version = "unstable";

    src = builtins.path {
      name = "${pname}-source";
      path = ./.;
    };

    nativeBuildInputs = with pkgs; [git cmake makeWrapper];
    buildInputs = runtimePackages;

    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    cargoSha256 = null;

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/${pname} \
        --prefix PATH : ${pkgs.lib.makeBinPath runtimePackages}
    '';
  }
