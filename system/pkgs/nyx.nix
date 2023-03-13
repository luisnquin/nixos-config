{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = name;
      rev = "a32054a3277ef1a1874319049fd306478d25ba2f";
      sha256 = "1xviqzwwylln16xzpm23x1xi1wb65nsyfgyqw7pw39zxk2lcgyqh";
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
