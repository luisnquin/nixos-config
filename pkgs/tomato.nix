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
      rev = "62a05d25a42897e2204bbf4f2e85eedce90398d0";
      sha256 = "1bw8wvbw47i3bbhnbg0nsljasqh61wkpx3xhlj75a67ycd7zihp3";
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
