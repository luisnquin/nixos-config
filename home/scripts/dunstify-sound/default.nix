{pkgs ? import <nixpkgs> {}}: let
  runtimePackages = with pkgs; [
    pulseaudio
    alsa-utils
    dunst
  ];
in
  pkgs.stdenv.mkDerivation rec {
    name = "dunstify-sound";

    src = builtins.path {
      inherit name;
      path = ./.;
    };

    nativeBuildInputs = with pkgs; [
      makeWrapper
    ];

    propagatedBuildInputs = runtimePackages;

    postPatch = ''
      substituteInPlace ./main.sh \
        --replace 'path/to/assets' '${placeholder "out"}/assets'
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/

      mkdir -p $out/bin/
      cp ./main.sh $out/bin/dunstify-sound
      chmod +x $out/bin/dunstify-sound

      mkdir -p $/out/assets/
      cp -r $src/assets/ $out/assets/

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/dunstify-sound \
        --prefix PATH : ${pkgs.lib.makeBinPath runtimePackages}
    '';
  }
