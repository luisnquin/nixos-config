{
  config,
  pkgs,
  lib,
  ...
}: let
  zaread = pkgs.stdenvNoCC.mkDerivation {
    pname = "zaread";
    version = "1.5.0-035b476";

    src = pkgs.fetchurl {
      url = "https://raw.githubusercontent.com/paoloap/zaread/035b476f8a64627f047f710bb04cadb3aa696b4d/zaread";
      hash = "sha256-naZG3YJSTlqt9t1RzIcDtzga5wnMwkNqnBnmJ7rqxY8=";
    };

    dontUnpack = true;
    nativeBuildInputs = [pkgs.makeWrapper];

    installPhase = ''
      runHook preInstall

      install -Dm755 "$src" "$out/bin/zaread"
      wrapProgram "$out/bin/zaread" \
        --prefix PATH : ${lib.makeBinPath [
        pkgs.coreutils
        pkgs.file
        pkgs.libreoffice
        pkgs.zathura
      ]}

      runHook postInstall
    '';

    meta.mainProgram = "zaread";
  };
in {
  home.packages = [
    pkgs.nautilus
    zaread
  ];

  programs.ranger = {
    enable = true;

    settings = {
      use_preview_script = true;
      preview_script = "${config.home.homeDirectory}/.config/ranger/scope.sh";

      preview_images = true;
      preview_images_method = "ueberzug";
    };

    rifle = [
      {
        condition = "ext wav";
        command = "${pkgs.pulseaudio}/bin/paplay \"$@\"";
      }
      {
        condition = "ext ico|bmp|gif|jpeg|jpg|png|svg|webp";
        command = "${lib.getExe pkgs.feh} \"$@\"";
      }
      {
        condition = "ext pdf";
        command = "${lib.getExe pkgs.zathura} --fork \"$@\"";
      }
      {
        condition = "ext avi|m4v|mkv|mov|mp4|webm";
        command = "${lib.getExe pkgs.vlc} \"$@\"";
      }
      {
        condition = "ext csv|doc|docm|docx|dotx|odp|ods|odt|pps|ppsx|ppt|pptm|pptx|rtf|xls|xlsb|xlsm|xlsx";
        command = "${lib.getExe zaread} \"$@\"";
      }
      {
        condition = "ext 3mf";
        command = ''${lib.getExe config.programs."3mf2stl".package} "$1" "''${1%.3mf}.stl" && ${pkgs.xdg-utils}/bin/xdg-open "''${1%.3mf}.stl"'';
      }
    ];
  };

  xdg.configFile."ranger/scope.sh" = {
    source = pkgs.writeShellScript "ranger-scope.sh" ''
      call_bat () {
        ${lib.getExe pkgs.bat} --color=always --paging=never --style=plain "$1"
      }

      FILE_PATH="''${1}"
      PV_WIDTH="''${2}"
      PV_HEIGHT="''${3}"
      FILE_EXTENSION="''${FILE_PATH##*.}"
      FILE_MIMETYPE=$(${lib.getExe pkgs.file} -b --mime-type "''${FILE_PATH}")

      case "$FILE_MIMETYPE" in
        text/*|application/json)
          call_bat "$FILE_PATH"
          exit 0
          ;;
      esac

      if [ -z "$FILE_EXTENSION" ]; then
        call_bat "$FILE_PATH"
        exit 0
      fi

      exit 1
    '';
    executable = true;
  };
}
