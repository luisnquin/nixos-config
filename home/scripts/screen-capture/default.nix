{pkgs ? import <nixpkgs> {}}: let
  runtimePackages = with pkgs; [
    libnotify
    xdotool
    xclip
    maim
  ];
in
  pkgs.stdenv.mkDerivation rec {
    name = "screen-capture";

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
        --replace '/path/to/icon' '${placeholder "out"}/assets/screenshot.jpg'
    '';

    installPhase = ''
      runHook preInstall

      mkdir -p $out/

      mkdir -p $out/bin/
      cp ./main.sh $out/bin/${name}
      chmod +x $out/bin/${name}

      mkdir -p $/out/assets/
      cp -r $src/assets/ $out/assets/

      runHook postInstall
    '';

    postInstall = ''
      wrapProgram ${placeholder "out"}/bin/${name} \
        --prefix PATH : ${pkgs.lib.makeBinPath runtimePackages}
    '';
  }
