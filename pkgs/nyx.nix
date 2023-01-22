{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
  repo = "nixos-config";
in
  pkgs.stdenv.mkDerivation {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = repo;
      rev = "466505379526d47c2177459cadc4c383d1e36a4b";
      sha256 = "0gw70h7cfsn416h0r9v291jy1v14vbvm1vs59bjsryifah18p90d";
    };

    installPhase = ''
      mkdir -p $out/bin/
      cp $src/.scripts/main.sh $out/bin/
      mv $out/bin/main.sh $out/bin/nyx
      chmod +x $out/bin/nyx
    '';

    propagatedBuildInputs = with pkgs; [
      alejandra
      exa
    ];

    meta = with pkgs.lib; {
      description = "A CLI tool to manage NixOS computers";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; ["${owner}"];
    };
  }
