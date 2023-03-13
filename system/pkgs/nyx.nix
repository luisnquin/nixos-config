{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
in
  pkgs.stdenv.mkDerivation rec {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = name;
      rev = "408d60eb6c0f5a30b941299ca8c380950d24026e";
      sha256 = "0lpwjqq9ws8j0nwknp8cql8db1i4brhywlavn0q14164ganki2qd";
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
