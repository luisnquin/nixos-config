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
      rev = "d0ee8ccc194be905ea01d5685bcd9011e1a89b00";
      sha256 = "12gqr57h4gjrn5is4izwvmhyqiqr063qbkcvbzn4kb1dj6kanbli";
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
