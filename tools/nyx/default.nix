{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = builtins.path {
      inherit name;
      path = ./.;
    };

    propagatedBuildInputs = with pkgs; [
      alejandra
      exa
    ];

    installPhase = ''
      mkdir -p $out/bin/
      cp $src/main.sh $out/bin/nyx
      chmod +x $out/bin/nyx
    '';

    meta = with pkgs.lib; {
      description = "A script to manage my NixOS computer";
      homepage = "https://github.com/${owner}/nix-config";
      license = licenses.unlicense;
      maintainers = with maintainers; [luisnquin];
    };
  }
