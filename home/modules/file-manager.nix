{
  config,
  pkgs,
  lib,
  ...
}: let
  magick = lib.getExe' pkgs.imagemagick "magick";
  rsvgConvert = lib.getExe' pkgs.librsvg "rsvg-convert";
  pdftoppm = lib.getExe' pkgs.poppler-utils "pdftoppm";
  ffmpegthumbnailer = lib.getExe pkgs.ffmpegthumbnailer;
  ffmpeg = lib.getExe' pkgs.ffmpeg "ffmpeg";
in {
  home.packages = [
    pkgs.nautilus
  ];

  programs.ranger = {
    enable = true;

    settings = {
      use_preview_script = true;
      preview_script = "${config.home.homeDirectory}/.config/ranger/scope.sh";

      preview_images = true;
      # ueberzug draws an override-redirect X window, which lands in XWayland
      # space and never over a native-Wayland ghostty surface. Ghostty speaks
      # the kitty graphics protocol, and ranger's kitty displayer falls back to
      # unicode placeholders under tmux (allow-passthrough is on).
      preview_images_method = "kitty";
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
      IMAGE_CACHE_PATH="''${4}"
      PV_IMAGE_ENABLED="''${5}"
      FILE_EXTENSION="''${FILE_PATH##*.}"
      FILE_EXTENSION_LOWER=$(printf '%s' "$FILE_EXTENSION" | tr '[:upper:]' '[:lower:]')
      FILE_MIMETYPE=$(${lib.getExe pkgs.file} -b --mime-type "''${FILE_PATH}")

      # Ranger reads a thumbnail from $IMAGE_CACHE_PATH on exit 6 and opens the
      # file itself on exit 7. Exit 7 is only safe for formats the displayer's
      # Pillow backend decodes on its own; everything else has to be rasterised
      # here first.
      thumb_ok () {
        [ -s "$IMAGE_CACHE_PATH" ]
      }

      thumb_magick () {
        ${magick} "$1[0]" -auto-orient -thumbnail '1920x1920>' \
          -background none png:"$IMAGE_CACHE_PATH" 2>/dev/null
        thumb_ok
      }

      thumb_svg () {
        ${rsvgConvert} --width=1200 --keep-aspect-ratio \
          --output="$IMAGE_CACHE_PATH" "$1" 2>/dev/null
        thumb_ok
      }

      thumb_pdf () {
        ${pdftoppm} -png -r 150 -f 1 -l 1 -singlefile "$1" "$IMAGE_CACHE_PATH" 2>/dev/null
        [ -s "$IMAGE_CACHE_PATH.png" ] && mv -f "$IMAGE_CACHE_PATH.png" "$IMAGE_CACHE_PATH"
        thumb_ok
      }

      thumb_video () {
        ${ffmpegthumbnailer} -i "$1" -o "$IMAGE_CACHE_PATH" -c png -s 1024 -q 8 2>/dev/null
        thumb_ok
      }

      thumb_cover () {
        ${ffmpeg} -loglevel quiet -y -i "$1" -map 0:v:0 -frames:v 1 \
          -c:v png -f image2 "$IMAGE_CACHE_PATH" 2>/dev/null
        thumb_ok
      }

      if [ "$PV_IMAGE_ENABLED" = "True" ]; then
        case "$FILE_EXTENSION_LOWER" in
          svg|svgz)
            thumb_svg "$FILE_PATH" && exit 6 ;;
          heic|heif|avif|jxl|psd|psb|xcf|cr2|cr3|nef|arw|dng|orf|rw2|raf|srw|pef|3fr|erf|kdc|mrw|x3f)
            thumb_magick "$FILE_PATH" && exit 6 ;;
        esac

        case "$FILE_MIMETYPE" in
          image/svg+xml)
            thumb_svg "$FILE_PATH" && exit 6 ;;
          image/png|image/jpeg|image/gif|image/bmp|image/x-ms-bmp|image/webp|image/tiff|image/x-icon|image/vnd.microsoft.icon|image/jp2|image/x-tga|image/x-pcx|image/x-xbitmap|image/x-portable-pixmap|image/x-portable-graymap|image/x-portable-bitmap|image/x-portable-anymap)
            exit 7 ;;
          image/*)
            thumb_magick "$FILE_PATH" && exit 6 ;;
          video/*)
            thumb_video "$FILE_PATH" && exit 6 ;;
          audio/*)
            thumb_cover "$FILE_PATH" && exit 6 ;;
          application/pdf|application/postscript)
            thumb_pdf "$FILE_PATH" && exit 6 ;;
        esac
      fi

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
