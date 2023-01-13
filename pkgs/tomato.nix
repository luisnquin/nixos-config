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
      rev = "11a152fc6c4bdd49b0c212ddf8c1721441270152";
      sha256 = "sha256-6N1GRgG4w62xfNlicqaRig9ETVFzGW0HyznRURlzfCE=";
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
