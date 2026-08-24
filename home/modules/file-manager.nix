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
  f3d = lib.getExe' pkgs.f3d "f3d";
  meshCameraFile = "${config.xdg.cacheHome}/ranger/mesh-camera";
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
        condition = "ext stl";
        command = "${lib.getExe' pkgs.fstl "fstl"} \"$@\"";
      }
      {
        condition = "ext 3mf";
        command = ''${lib.getExe config.programs."3mf2stl".package} "$1" "''${1%.3mf}.stl" && ${pkgs.xdg-utils}/bin/xdg-open "''${1%.3mf}.stl"'';
      }
    ];
  };

  xdg.configFile."ranger/commands.py".text = ''
    import atexit
    import curses
    import os
    import sys
    import time

    import ranger
    from ranger.api.commands import Command
    from ranger.gui.displayable import DisplayableContainer
    from ranger.gui.mouse_event import MouseEvent
    from ranger.gui.ui import UI

    CAMERA_FILE = "${meshCameraFile}"
    DEFAULT_CAMERA = (0.0, 0.0, 0.85)
    MESH_EXTENSIONS = (
        ".stl", ".3mf", ".obj", ".ply", ".off",
        ".gltf", ".glb", ".dae", ".fbx", ".3ds",
        ".vtk", ".vtu", ".vtp",
    )

    DEGREES_PER_COLUMN = 3.0
    DEGREES_PER_ROW = 5.0
    WHEEL_ZOOM_STEP = 1.15
    # A render costs roughly half a second, so an unthrottled drag would queue
    # far more of them than it can retire.
    MIN_RENDER_INTERVAL = 0.25
    # ncurses only turns on 1006;1000, which reports buttons but never motion,
    # so drags stay invisible until we ask for 1002 ourselves.
    MOTION_MODE = "\x1b[?1002"

    drag = {"active": False, "x": 0, "y": 0, "azimuth": 0.0, "elevation": 0.0, "at": 0.0}


    def read_camera():
        try:
            with open(CAMERA_FILE, encoding="utf-8") as handle:
                azimuth, elevation, zoom = handle.read().split()[:3]
            return float(azimuth), float(elevation), float(zoom)
        except (OSError, ValueError):
            return DEFAULT_CAMERA


    def write_camera(camera):
        os.makedirs(os.path.dirname(CAMERA_FILE), exist_ok=True)
        with open(CAMERA_FILE, "w", encoding="utf-8") as handle:
            handle.write("{0:.6g} {1:.6g} {2:.6g}\n".format(*camera))


    def clamp_camera(azimuth, elevation, zoom):
        return (
            azimuth % 360.0,
            max(-89.0, min(89.0, elevation)),
            max(0.05, min(20.0, zoom)),
        )


    def is_mesh(fobj):
        return (
            fobj is not None
            and fobj.is_file
            and fobj.relative_path.lower().endswith(MESH_EXTENSIONS)
        )


    def cached_render(fm, fobj):
        return os.path.join(
            ranger.args.cachedir,
            fm.sha512_encode(fobj.realpath, inode=getattr(fobj.stat, "st_ino", None)),
        )


    def apply_camera(fm, camera):
        write_camera(camera)

        fobj = fm.thisfile
        if not is_mesh(fobj):
            return

        # get_preview() serves the cached render whenever it is newer than the
        # mesh, so the file has to go before the in-memory entry does.
        try:
            os.remove(cached_render(fm, fobj))
        except OSError:
            pass

        fm.update_preview(fobj.realpath)
        fm.ui.need_redraw = True


    def render_pending(fm, fobj):
        return fm.previews.get(fobj.realpath, {}).get("loading", False)


    def preview_column(ui):
        columns = getattr(ui.browser, "columns", None)
        if not columns:
            return None
        column = columns[-1]
        return column if column.visible else None


    def orbit(ui, event, final):
        """Rotate to the pointer's offset from where the drag started."""
        fobj = ui.fm.thisfile
        if not is_mesh(fobj):
            drag["active"] = False
            return

        if (event.x, event.y) == (drag["x"], drag["y"]):
            return

        now = time.monotonic()
        if not final and (
            now - drag["at"] < MIN_RENDER_INTERVAL or render_pending(ui.fm, fobj)
        ):
            return

        drag["at"] = now
        apply_camera(
            ui.fm,
            clamp_camera(
                drag["azimuth"] - (event.x - drag["x"]) * DEGREES_PER_COLUMN,
                drag["elevation"] - (event.y - drag["y"]) * DEGREES_PER_ROW,
                read_camera()[2],
            ),
        )


    def handle_mesh_mouse(ui, event):
        """Orbit and zoom the 3D preview; returns True when the event is consumed."""
        moving = bool(event.bstate & curses.REPORT_MOUSE_POSITION)
        released = bool(event.bstate & curses.BUTTON1_RELEASED)

        # A drag keeps the pointer captured, so leaving the preview mid-rotation
        # neither drops the gesture nor leaks it into the file list.
        if drag["active"]:
            if moving or released:
                orbit(ui, event, final=released)
                drag["active"] = not released
                return True
            drag["active"] = False

        column = preview_column(ui)
        if column is None or not column.contains_point(event.y, event.x):
            return False

        if not is_mesh(ui.fm.thisfile):
            return False

        # REPORT_MOUSE_POSITION is above ALL_MOUSE_EVENTS, which ranger reads as
        # an invalid button and therefore as a scroll: motion goes first.
        if moving:
            return True

        if event.pressed(3):
            apply_camera(ui.fm, DEFAULT_CAMERA)
            return True

        direction = event.mouse_wheel_direction()
        if direction:
            azimuth, elevation, zoom = read_camera()
            factor = WHEEL_ZOOM_STEP if direction < 0 else 1.0 / WHEEL_ZOOM_STEP
            apply_camera(ui.fm, clamp_camera(azimuth, elevation, zoom * factor))
            return True

        if event.pressed(1):
            azimuth, elevation, _ = read_camera()
            drag.update(
                active=True,
                x=event.x,
                y=event.y,
                azimuth=azimuth,
                elevation=elevation,
                at=0.0,
            )
            return True

        return False


    def handle_mouse(ui):
        """Replaces UI.handle_mouse: curses.getmouse() may only be read once."""
        try:
            event = MouseEvent(curses.getmouse())
        except curses.error:
            return

        if ui.console.visible:
            return

        if handle_mesh_mouse(ui, event):
            return

        if event.bstate & curses.REPORT_MOUSE_POSITION:
            return

        DisplayableContainer.click(ui, event)


    def motion_reporting(enabled):
        try:
            sys.__stdout__.write(MOTION_MODE + ("h" if enabled else "l"))
            sys.__stdout__.flush()
        except (OSError, ValueError):
            pass


    def initialize(ui):
        original_initialize(ui)
        if ui.settings.mouse_enabled:
            motion_reporting(True)


    def suspend(ui):
        motion_reporting(False)
        original_suspend(ui)


    original_initialize = UI.initialize
    original_suspend = UI.suspend

    UI.handle_mouse = handle_mouse
    UI.initialize = initialize
    UI.suspend = suspend

    # 1002 outlives the alternate screen, so a crash would leave the shell
    # swallowing mouse reports.
    atexit.register(motion_reporting, False)


    class mesh_camera(Command):
        """:mesh_camera <azimuth> <elevation> <zoom> | reset

        Orbit and zoom the 3D preview. Azimuth and elevation are degrees added
        to the current camera, zoom is a multiplier applied to it.
        """

        def execute(self):
            if self.arg(1) == "reset":
                camera = DEFAULT_CAMERA
            else:
                azimuth, elevation, zoom = read_camera()
                camera = clamp_camera(
                    azimuth + self.number_arg(1, 0.0),
                    elevation + self.number_arg(2, 0.0),
                    zoom * self.number_arg(3, 1.0),
                )

            self.fm.notify(
                "mesh camera: azimuth {0:.0f} elevation {1:.0f} zoom {2:.2f}".format(*camera)
            )
            apply_camera(self.fm, camera)

        def number_arg(self, index, fallback):
            try:
                return float(self.arg(index))
            except (TypeError, ValueError):
                return fallback
  '';

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

      # The camera is shared state with the ranger `mesh_camera` command: it
      # rewrites this file and drops both preview caches, so the next scope run
      # renders the same mesh from the new angle.
      thumb_mesh () {
        az=0; el=0; zoom=0.85
        if [ -r "${meshCameraFile}" ]; then
          read -r az el zoom < "${meshCameraFile}" || true
        fi

        ${f3d} "$1" --output="$IMAGE_CACHE_PATH" --resolution=900,900 \
          --up=+Z --camera-direction=-1,-0.6,-0.4 \
          --camera-azimuth-angle="$az" --camera-elevation-angle="$el" \
          --camera-zoom-factor="$zoom" \
          --ambient-occlusion --no-background >/dev/null 2>&1
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
          stl|3mf|obj|ply|off|gltf|glb|dae|fbx|3ds|vtk|vtu|vtp)
            thumb_mesh "$FILE_PATH" && exit 6 ;;
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
