{
  config,
  pkgs,
  lib,
  ...
}: {
  home.packages = [
    pkgs.nautilus
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
        condition = "ext ico|bmp|gif|jpeg|jpg|png|svg|webp|heic";
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
      # {
      #   condition = "ext csv|doc|docm|docx|dotx|odp|ods|odt|pps|ppsx|ppt|pptm|pptx|rtf|xls|xlsb|xlsm|xlsx";
      #   command = "${lib.getExe zaread} \"$@\"";
      # }
      {
        condition = "ext 3mf";
        command = ''${lib.getExe config.programs."3mf2stl".package} "$1" "''${1%.3mf}.stl" && ${pkgs.xdg-utils}/bin/xdg-open "''${1%.3mf}.stl"'';
      }
    ];
  };

  xdg.configFile."ranger/scope.sh" = {
    source = pkgs.writeShellScript "ranger-scope.sh" ''
      call_bat () {
        if [ -n "$2" ]; then
          ${lib.getExe pkgs.bat} --color=always --paging=never --style=plain --language="$2" "$1"
        else
          ${lib.getExe pkgs.bat} --color=always --paging=never --style=plain "$1"
        fi
      }

      FILE_PATH="''${1}"
      PV_WIDTH="''${2}"
      PV_HEIGHT="''${3}"
      FILE_EXTENSION="''${FILE_PATH##*.}"
      FILE_EXTENSION_LOWER=$(printf '%s' "$FILE_EXTENSION" | tr '[:upper:]' '[:lower:]')
      FILE_MIMETYPE=$(${lib.getExe pkgs.file} -b --mime-type "''${FILE_PATH}")

      case "$FILE_EXTENSION_LOWER" in
        tsx)  call_bat "$FILE_PATH" tsx;  exit 0 ;;
        ts|mts|cts) call_bat "$FILE_PATH" ts; exit 0 ;;
        jsx)  call_bat "$FILE_PATH" jsx;  exit 0 ;;
        js|mjs|cjs) call_bat "$FILE_PATH" js; exit 0 ;;
      esac

      case "$FILE_MIMETYPE" in
        text/*|application/json|application/javascript|application/x-javascript|application/typescript|application/xml|application/x-shellscript|application/x-yaml)
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
