{
  fetchgit,
  stdenv,
  pkgs,
  lib,
}: let
  repository-url = "https://github.com/gabrielzschmitz/Tomato.C";
in
  stdenv.mkDerivation {
    name = "tomato";

    src = fetchgit {
      url = repository-url;
      rev = "3bf8d16598e566a12e7b8e3e3a63217c5698a3ea";
      sha256 = "1jdkh2xiin96sxslngvma6gybibq4wwww61rv4ijh5l6bd82sy0a";
    };

    installPhase = ''
      mkdir -p $out/bin && cp tomato $out/bin/

      ln -s $(which notify-send) $out/bin/
      ln -s $(which mpv) $out/bin/
    '';

    buildInputs = with pkgs; [
      gcc
      which
      gnumake
      ncurses
      pkgconfig
    ];

    propagatedBuildInputs = with pkgs; [
      libnotify
      mpv
    ];

    meta = with lib; {
      description = "A pomodoro timer written in pure C.";
      homepage = repository-url;
      license = licenses.gpl3Plus;
      maintainers = with maintainers; [luisnquin];
    };
  }
