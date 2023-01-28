{pkgs ? import <nixpkgs> {}}: let
  owner = "luisnquin";
  repo = "nixos-config";
in
  pkgs.stdenv.mkDerivation {
    name = "nyx";
    src = pkgs.fetchFromGitHub {
      owner = owner;
      repo = repo;
      rev = "9cfc7ad094047069b45d9deadfce7634700eb963";
      sha256 = "0vlx4hzax2qk69lzkf0l9zr6n775ia9f4bpznxk99jcx21k0gi9n";
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
