{pkgs ? import <nixpkgs> {}}: let
  repo-owner = "faressoft";
in
  pkgs.buildNpmPackage rec {
    pname = "terminalizer";
    version = "0.9.0";

    src = pkgs.fetchFromGitHub {
      owner = repo-owner;
      repo = pname;
      rev = "dc65dbebf124419b200394e6f1fffea0d6090a9c";
      sha256 = "0ldfhvd7y9fs3nn1ijc6xmh7gvvq75hzrhark5m9a9qclarfs4si";
    };

    npmDepsHash = "sha256-Tz1l0SNz5uaNDtLxILRBskOI8+Ls90cVUzPyfJ/VBcw=";
    npmPackFlags = ["--ignore-scripts"];

    meta = with pkgs.lib; {
      description = "Record your terminal and generate animated gif images or share a web player";
      homepage = "https://github.com/${repo-owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; [luisnquin];
    };
  }
