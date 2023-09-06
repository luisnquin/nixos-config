{pkgs ? import <nixpkgs> {}}: let
  runtimePackages = with pkgs; [
    dbus
  ];
in
  pkgs.stdenv.mkDerivation rec {
    name = "spotify-dbus";

    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = [pkgs.makeWrapper];

    propagatedBuildInputs = runtimePackages;

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin/
      cp ./main.sh $out/bin/${name}
      chmod +x $out/bin/${name}

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/${name} \
        --prefix PATH : ${pkgs.lib.makeBinPath runtimePackages}
    '';
  }
