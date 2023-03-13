{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = name;
      rev = "fd2612b998043dd025f0fbb30da74bb2ef2c605d";
      sha256 = "1si58criblgv9r65agynglrdxgyszpv0rydmssmj2r9ghy3wvnii";
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
