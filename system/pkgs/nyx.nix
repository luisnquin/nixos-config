{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = name;
      rev = "999506560720fa32a1f57177941fc75d18f5b3ad";
      sha256 = "16iybxsmms1mky2cq2i714g2ysifz8jjxqlb7xba23qxwcgmfjay";
    };

    installPhase = ''
      mkdir -p $out/bin/
      cp $src/nyx.sh $out/bin/
      mv $out/bin/nyx.sh $out/bin/nyx
      chmod +x $out/bin/nyx
    '';

    propagatedBuildInputs = with pkgs; [
      alejandra
      exa
    ];

    meta = with pkgs.lib; {
      description = "A CLI tool to manage NixOS computers";
      homepage = "https://github.com/${owner}/${name}";
      license = licenses.unlicense;
      maintainers = with maintainers; [luisnquin];
    };
  }
