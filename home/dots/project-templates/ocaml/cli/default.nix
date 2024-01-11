{pkgs ? import <nixpkgs> {}, ...}:
with pkgs;
  ocamlPackages.buildDunePackage rec {
    pname = "cli_app";
    version = "unstable";

    src = builtins.path {
      name = "${pname}-source";
      path = ./.;
    };

    # checkInputs = [alcotest ppx_let];
    # buildInputs = [ocaml-syntax-shims];
    propagatedBuildInputs = [ocamlPackages.result];
    doCheck = lib.versionAtLeast ocaml.version "5.0.0";
  }
