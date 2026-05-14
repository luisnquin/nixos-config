# https://wiki.hypr.land/Configuring/Basics/Binds/
{
  config,
  pkgs,
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

  cliphistFuzzel = pkgs.writeShellScript "hypr-cliphist-fuzzel" ''
    ${lib.getExe pkgs.scripts.cliphist-rofi} | ${lib.getExe pkgs.fuzzel} --dmenu | cliphist decode | wl-copy
  '';

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

  b = keys: dsp: {
    _args = [keys dsp];
  };

  bm = keys: dsp: {
    _args = [keys dsp {mouse = true;}];
  };
in [
  (b "${mainMod} + left" (mkLuaInline "hl.dsp.focus({ direction = \"left\" })"))
  (b "${mainMod} + right" (mkLuaInline "hl.dsp.focus({ direction = \"right\" })"))
  (b "${mainMod} + up" (mkLuaInline "hl.dsp.focus({ direction = \"up\" })"))
  (b "${mainMod} + down" (mkLuaInline "hl.dsp.focus({ direction = \"down\" })"))

  (b "${mainMod} + 1" (mkLuaInline "hl.dsp.focus({ workspace = 1 })"))
  (b "${mainMod} + 2" (mkLuaInline "hl.dsp.focus({ workspace = 2 })"))
  (b "${mainMod} + 3" (mkLuaInline "hl.dsp.focus({ workspace = 3 })"))
  (b "${mainMod} + 4" (mkLuaInline "hl.dsp.focus({ workspace = 4 })"))
  (b "${mainMod} + 5" (mkLuaInline "hl.dsp.focus({ workspace = 5 })"))
  (b "${mainMod} + 6" (mkLuaInline "hl.dsp.focus({ workspace = 6 })"))
  (b "${mainMod} + 7" (mkLuaInline "hl.dsp.focus({ workspace = 7 })"))
  (b "${mainMod} + 8" (mkLuaInline "hl.dsp.focus({ workspace = 8 })"))
  (b "${mainMod} + 9" (mkLuaInline "hl.dsp.focus({ workspace = 9 })"))
  (b "${mainMod} + 0" (mkLuaInline "hl.dsp.focus({ workspace = 10 })"))

  (b "${mainMod} + SHIFT + 1" (mkLuaInline "hl.dsp.window.move({ workspace = 1 })"))
  (b "${mainMod} + SHIFT + 2" (mkLuaInline "hl.dsp.window.move({ workspace = 2 })"))
  (b "${mainMod} + SHIFT + 3" (mkLuaInline "hl.dsp.window.move({ workspace = 3 })"))
  (b "${mainMod} + SHIFT + 4" (mkLuaInline "hl.dsp.window.move({ workspace = 4 })"))
  (b "${mainMod} + SHIFT + 5" (mkLuaInline "hl.dsp.window.move({ workspace = 5 })"))
  (b "${mainMod} + SHIFT + 6" (mkLuaInline "hl.dsp.window.move({ workspace = 6 })"))
  (b "${mainMod} + SHIFT + 7" (mkLuaInline "hl.dsp.window.move({ workspace = 7 })"))
  (b "${mainMod} + SHIFT + 8" (mkLuaInline "hl.dsp.window.move({ workspace = 8 })"))
  (b "${mainMod} + SHIFT + 9" (mkLuaInline "hl.dsp.window.move({ workspace = 9 })"))
  (b "${mainMod} + SHIFT + 0" (mkLuaInline "hl.dsp.window.move({ workspace = 10 })"))

  (b "${mainMod} + SHIFT + h" (mkLuaInline "hl.dsp.window.resize({ x = -40, y = 0, relative = true })"))
  (b "${mainMod} + SHIFT + l" (mkLuaInline "hl.dsp.window.resize({ x = 40, y = 0, relative = true })"))
  (b "${mainMod} + SHIFT + k" (mkLuaInline "hl.dsp.window.resize({ x = 0, y = -40, relative = true })"))
  (b "${mainMod} + SHIFT + j" (mkLuaInline "hl.dsp.window.resize({ x = 0, y = 40, relative = true })"))

  (b "${mainMod} + mouse_down" (mkLuaInline "hl.dsp.focus({ workspace = \"e+1\" })"))
  (b "${mainMod} + mouse_up" (mkLuaInline "hl.dsp.focus({ workspace = \"e-1\" })"))

  (b "${mainMod} + SPACE" (mkLuaInline "hl.dsp.window.float({ action = \"toggle\" })"))
  (b "${mainMod} + S" (mkLuaInline "hl.dsp.layout(\"togglesplit\")"))

  (b "${mainMod} + SHIFT + W" (dspExec "${./kill-active.sh}"))
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
  (b "${mainMod} + SHIFT + C" (dspExec (toString cliphistFuzzel)))

  (b "CTRL + SHIFT + braceleft" (dspExec (pctlFallback "position 5-")))
  (b "CTRL + SHIFT + braceright" (dspExec (pctlFallback "position 5+")))
  (b "${mainMod} + SHIFT + braceright" (dspExec (pctlFallback "next")))
  (b "${mainMod} + SHIFT + braceleft" (dspExec (pctlFallback "previous")))
  (b "${mainMod} + Pause" (dspExec "${pctlCmd} play-pause --player=spotify"))
  (b "${mainMod} + Delete" (dspExec "${pctlCmd} play-pause --all-players --ignore-player=spotify"))

  (bm "${mainMod} + mouse:272" (mkLuaInline "hl.dsp.window.drag()"))
  (bm "${mainMod} + mouse:273" (mkLuaInline "hl.dsp.window.resize()"))
]
