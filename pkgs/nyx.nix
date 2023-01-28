{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
  repo = "nixos-config";
in
  pkgs.stdenv.mkDerivation {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = repo;
      rev = "da63baae2fe091fa8ab7aae83ff82e8502127ae2";
      sha256 = "1ajp7xm8x7jqq9wqzwb7qi755507i2sc3i1acwa2rxarr2s4p4q0";
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
