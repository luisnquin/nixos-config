{pkgs ? import <nixpkgs> {}}: let
  inherit (pkgs) lib;
in
  pkgs.stdenv.mkDerivation rec {
    name = "tplr";

    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = [pkgs.makeWrapper];

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin

      cp ./main.sh $out/bin/${name}
      chmod +x $out/bin/${name}

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/${name} \
        --prefix PATH : ${lib.makeBinPath [pkgs.coreutils]}
    '';
  }
