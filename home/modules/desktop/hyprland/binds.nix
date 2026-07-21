# https://wiki.hypr.land/Configuring/Basics/Binds/
{
  config,
  pkgs,
  libx,
  lib,
  ...
}: let
  inherit (lib.generators) mkLuaInline;

  mainMod = "SUPER";
  hyprctlCmd = "${pkgs.hyprland}/bin/hyprctl";

  raffiExec = pkgs.writeShellScript "hypr-raffi-exec" ''
    val="$(raffi -pI)"
    [ -n "$val" ] && exec ${hyprctlCmd} dispatch exec "$val"
  '';

  clipz = {
    toggleMode = pkgs.writeShellScript "hypr-clipz-toggle" (
      let
        notify = message:
          libx.notify.desktop {
            image = ./crop.512.png;
            title = "Clipboard";
            inherit message;
          };
      in ''
        mode="$(${lib.getExe config.programs.cliphizt.package} mode toggle)"
        case "$mode" in
          normal)
            ${notify "Entries will be kept forever."}
            ;;
          ephemeral)
            ${notify "Entries will expire after the configured TTL."}
            ;;
          single-use)
            ${notify "Entries will be deleted after first use."}
            ;;
          *)
            ${notify "Mode changed: $mode"}
            ;;
        esac
      ''
    );

    lens = pkgs.writeShellApplication {
      name = "hypr-clipz-lens";

      runtimeInputs = with pkgs; [
        wl-clipboard
        cliphizt
        cliplenz
      ];

      text = ''
        selected="$(cliphizt list | cliplenz --preview 'cliphizt decode')"

        if [ -n "$selected" ]; then
          id="$(printf '%s\n' "$selected" | awk '{print $1}')"
          printf '%s\n' "$id" | cliphizt decode | wl-copy
        fi
      '';
    };
  };

  grimblastCmd = let
    inherit (pkgs.lib) getExe;
    package = pkgs.grimblast.overrideAttrs (_oldAttrs: {
      prePatch = ''
        substituteInPlace ./grimblast --replace '-t 3000' '-t 3000 -i ${./crop.512.png}'
      '';
    });
  in
    getExe package;

  hyprPrintScreen = let
    inherit (pkgs.lib) getExe;
    askTargetCmd = "${getExe pkgs.hyprshot} -m output --clipboard-only";
    allTargetsCmd = "${grimblastCmd} --notify copy screen";
    evalCmd = "${hyprctlCmd} monitors all -j | ${getExe pkgs.jq} \". | length\"";
  in
    pkgs.writeShellScript "hypr-print-screen" ''
      if [ "$(${evalCmd})" -eq 1 ]; then
        exec ${allTargetsCmd}
      else
        exec ${askTargetCmd}
      fi
    '';

  pctlCmd = lib.getExe pkgs.playerctl;
  pctlFallback = cmd: "bash -c 'if ${pctlCmd} --player=spotify status 2>/dev/null | grep -q Playing; then ${pctlCmd} ${cmd} --player=spotify; else ${pctlCmd} ${cmd}; fi'";

  inherit (pkgs.scripts) sys-sound sys-brightness;

  toLua' = lib.generators.toLua {};

  dspExec = cmd: mkLuaInline ("hl.dsp.exec_cmd(" + toLua' cmd + ")");

  layoutMsg = msg: mkLuaInline ("hl.dsp.layout(" + toLua' msg + ")");

  focusWs = ws: mkLuaInline "hl.dsp.focus({ workspace = ${toLua' ws} })";

  moveWs = ws: mkLuaInline "hl.dsp.window.move({ workspace = ${toLua' ws} })";

  focusDir = direction: mkLuaInline "hl.dsp.focus({ direction = ${toLua' direction} })";

  b = keys: dsp: {
    _args = [keys dsp];
  };

  bm = keys: dsp: {
    _args = [keys dsp {mouse = true;}];
  };

  wsKeys = ["1" "2" "3" "4" "5" "6" "7" "8" "9" "0"];
in
  [
    (b "${mainMod} + left" (focusDir "left"))
    (b "${mainMod} + right" (focusDir "right"))
    (b "${mainMod} + up" (focusDir "up"))
    (b "${mainMod} + down" (focusDir "down"))
  ]
  ++ lib.imap1 (ws: key: b "${mainMod} + ${key}" (focusWs ws)) wsKeys
  ++ [
    (b "${mainMod} + apostrophe" (focusWs "name:~"))
  ]
  ++ lib.imap1 (ws: key: b "${mainMod} + SHIFT + ${key}" (moveWs ws)) wsKeys
  ++ [
    (b "${mainMod} + SHIFT + apostrophe" (moveWs "name:~"))

    (b "${mainMod} + SHIFT + h" (mkLuaInline "hl.dsp.window.resize({ x = -40, y = 0, relative = true })"))
    (b "${mainMod} + SHIFT + l" (mkLuaInline "hl.dsp.window.resize({ x = 40, y = 0, relative = true })"))
    (b "${mainMod} + SHIFT + k" (mkLuaInline "hl.dsp.window.resize({ x = 0, y = -40, relative = true })"))
    (b "${mainMod} + SHIFT + j" (mkLuaInline "hl.dsp.window.resize({ x = 0, y = 40, relative = true })"))

    (b "${mainMod} + mouse_down" (focusWs "e+1"))
    (b "${mainMod} + mouse_up" (focusWs "e-1"))

    (b "${mainMod} + SPACE" (mkLuaInline "hl.dsp.window.float({ action = \"toggle\" })"))
    (b "${mainMod} + S" (layoutMsg "togglesplit"))

    # scrolling layout: https://wiki.hypr.land/Configuring/Layouts/Scrolling-Layout/
    (b "${mainMod} + comma" (layoutMsg "move -col"))
    (b "${mainMod} + period" (layoutMsg "move +col"))
    (b "${mainMod} + SHIFT + comma" (layoutMsg "swapcol l"))
    (b "${mainMod} + SHIFT + period" (layoutMsg "swapcol r"))
    (b "${mainMod} + CTRL + comma" (layoutMsg "colresize -conf"))
    (b "${mainMod} + CTRL + period" (layoutMsg "colresize +conf"))
    (b "${mainMod} + E" (layoutMsg "consume_or_expel next"))
    (b "${mainMod} + G" (layoutMsg "fit expand"))

    (b "${mainMod} + SHIFT + W" (mkLuaInline "hl.dsp.window.close()"))
    (b "${mainMod} + SHIFT + MINUS" (mkLuaInline "hl.dsp.window.pseudo({ action = \"toggle\" })"))
    (b "${mainMod} + F" (mkLuaInline "hl.dsp.window.fullscreen({ action = \"toggle\" })"))

    (b "${mainMod} + RETURN" (dspExec (lib.getExe config.programs.ghostty.package)))
    (b "XF86AudioMicMute" (dspExec "${lib.getExe sys-sound} --toggle-mic"))
    (b "XF86AudioMute" (dspExec "${lib.getExe sys-sound} --toggle-vol"))
    (b "XF86AudioLowerVolume" (dspExec "${lib.getExe sys-sound} --dec"))
    (b "XF86AudioRaiseVolume" (dspExec "${lib.getExe sys-sound} --inc"))
    (b "SUPER + XF86AudioRaiseVolume" (dspExec "${lib.getExe sys-sound} --inc --unleashed"))
    (b "XF86MonBrightnessDown" (dspExec "${lib.getExe sys-brightness} --dec"))
    (b "XF86MonBrightnessUp" (dspExec "${lib.getExe sys-brightness} --inc"))
    (b "${mainMod} + SHIFT + Q" (dspExec "${lib.getExe pkgs.fuzzel} --dmenu"))
    (b "${mainMod} + X" (dspExec (toString raffiExec)))
    (b "${mainMod} + Q" (dspExec (lib.getExe pkgs.fuzzel)))
    (b "${mainMod} + SHIFT + E" (dspExec (lib.getExe pkgs.bemoji)))
    (b "${mainMod} + SHIFT + R" (dspExec "${hyprctlCmd} reload"))
    (b "${mainMod} + M" (dspExec (lib.getExe pkgs.hyprstfu)))
    (b "${mainMod} + SHIFT + M" (dspExec "${lib.getExe pkgs.hyprstfu} -unmute-all"))
    (b "${mainMod} + SHIFT + XF86AudioLowerVolume" (dspExec "${lib.getExe pkgs.hyprstfu} -volume 5-"))
    (b "${mainMod} + SHIFT + XF86AudioRaiseVolume" (dspExec "${lib.getExe pkgs.hyprstfu} -volume 5+"))
    (b "${mainMod} + K" (dspExec "${lib.getExe pkgs.hyprdrop} -i ghostty.hyprdrop \"ghostty --class=ghostty.hyprdrop\""))

    (b "${mainMod} + SHIFT + Print" (dspExec "${grimblastCmd} --freeze --notify copy area"))
    (b "${mainMod} + Print" (dspExec "${grimblastCmd} --notify copy active"))
    (b "Print" (dspExec (toString hyprPrintScreen)))
    (b "${mainMod} + SHIFT + O" (dspExec (toString clipz.toggleMode)))
    (b "${mainMod} + SHIFT + C" (dspExec (lib.getExe clipz.lens)))

    (b "CTRL + SHIFT + braceleft" (dspExec (pctlFallback "position 5-")))
    (b "CTRL + SHIFT + braceright" (dspExec (pctlFallback "position 5+")))
    (b "${mainMod} + SHIFT + braceright" (dspExec (pctlFallback "next")))
    (b "${mainMod} + SHIFT + braceleft" (dspExec (pctlFallback "previous")))
    (b "${mainMod} + Pause" (dspExec "${pctlCmd} play-pause --player=spotify"))
    (b "${mainMod} + Delete" (dspExec "${pctlCmd} play-pause --all-players --ignore-player=spotify"))

    (bm "${mainMod} + mouse:272" (mkLuaInline "hl.dsp.window.drag()"))
    (bm "${mainMod} + mouse:273" (mkLuaInline "hl.dsp.window.resize()"))
  ]
