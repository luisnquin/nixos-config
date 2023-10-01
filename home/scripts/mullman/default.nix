{pkgs ? import <nixpkgs> {}}:
with pkgs;
with pkgs.ocamlPackages;
  buildDunePackage rec {
    pname = "mullman";
    version = "unstable";

    src = builtins.path {
      name = pname;
      path = ./.;
    };

    # checkInputs = [alcotest ppx_let];
    # buildInputs = [ocaml-syntax-shims];
    propagatedBuildInputs = [result];
    doCheck = lib.versionAtLeast ocaml.version "5.0.0";
  }
