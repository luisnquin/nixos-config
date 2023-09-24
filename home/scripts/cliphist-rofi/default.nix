{pkgs ? import <nixpkgs> {}}: let
  runtimePackages = [pkgs.cliphist];
in
  pkgs.stdenv.mkDerivation rec {
    name = "cliphist-rofi";

    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = with pkgs; [
      makeWrapper
    ];

    propagatedBuildInputs = runtimePackages;

    postPatch = ''
      substituteInPlace ./main.bash \
        --replace 'cliphist' '${pkgs.cliphist}/bin/cliphist'
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/bin/
      cp ./main.bash $out/bin/${name}
      chmod +x $out/bin/${name}

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/${name} \
        --prefix PATH : ${pkgs.lib.makeBinPath runtimePackages}
    '';
  }
