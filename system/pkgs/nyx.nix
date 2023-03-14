{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = name;
      rev = "5f070d5e073dfe174b2bd4753a228e0a6159aa89";
      sha256 = "05c3j4f9w8xi3bjk20za0zzqlzygd85mkmha6nby28zpbxbsar74";
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
