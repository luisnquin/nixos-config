{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
  repo = "nixos-config";
in
  pkgs.stdenv.mkDerivation {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = repo;
      rev = "9a6cb5f53a6c85fc6c8d67bea99d56b8bdee25a5";
      sha256 = "1irwvir4jqxipd3a8z3klk771h6nywsplf9659xqd19dw6961n4m";
    };

    installPhase = ''
      mkdir -p $out/bin/
      cp $src/.scripts/main.sh $out/bin/
      mv $out/bin/main.sh $out/bin/nyx
      chmod +x $out/bin/nyx
    '';

    meta = with pkgs.lib; {
      description = "A CLI tool to manage NixOS computers";
      homepage = "https://github.com/${owner}/${pname}";
      license = licenses.mit;
      maintainers = with maintainers; ["${owner}"];
    };
  }
