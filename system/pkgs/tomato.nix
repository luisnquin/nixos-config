{pkgs ? import <nixpkgs> {}}: let
  repository-url = "https://github.com/gabrielzschmitz/Tomato.C";
in
  pkgs.stdenv.mkDerivation {
    name = "tomato";

    src = pkgs.fetchgit {
      url = repository-url;
      rev = "ed75e6b4ddbb016ede6120bee80761c305e05747";
      sha256 = "1fd2alaanf20ywdcndrm49gl4s10nv7gc8mzlfzwi1gxlfgggb7b";
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

    meta = with pkgs.lib; {
      description = "A pomodoro timer written in pure C.";
      homepage = repository-url;
      license = licenses.gpl3Plus;
      maintainers = with maintainers; [luisnquin];
    };
  }
