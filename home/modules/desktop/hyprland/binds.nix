# https://wiki.hyprland.org/Configuring/Keywords/
{
  pkgs-extra,
  config,
  inputs,
  system,
  pkgs,
  ...
}: let
  mainMod = "SUPER";

  window = [
    "${mainMod}, left, movefocus, l"
    "${mainMod}, right, movefocus, r"
    "${mainMod}, up, movefocus, u"
    "${mainMod}, down, movefocus, d"
  ];

  workspaces = [
    # Go-to
    "${mainMod}, 1, workspace, 1"
    "${mainMod}, 2, workspace, 2"
    "${mainMod}, 3, workspace, 3"
    "${mainMod}, 4, workspace, 4"
    "${mainMod}, 5, workspace, 5"
    "${mainMod}, 6, workspace, 6"
    "${mainMod}, 7, workspace, 7"
    "${mainMod}, 8, workspace, 8"
    "${mainMod}, 9, workspace, 9"
    "${mainMod}, 0, workspace, 10"

    # Move window
    "${mainMod} SHIFT, 1, movetoworkspace, 1"
    "${mainMod} SHIFT, 2, movetoworkspace, 2"
    "${mainMod} SHIFT, 3, movetoworkspace, 3"
    "${mainMod} SHIFT, 4, movetoworkspace, 4"
    "${mainMod} SHIFT, 5, movetoworkspace, 5"
    "${mainMod} SHIFT, 6, movetoworkspace, 6"
    "${mainMod} SHIFT, 7, movetoworkspace, 7"
    "${mainMod} SHIFT, 8, movetoworkspace, 8"
    "${mainMod} SHIFT, 9, movetoworkspace, 9"
    "${mainMod} SHIFT, 0, movetoworkspace, 10"

    # Resize window
    "${mainMod} SHIFT, h, resizeactive, -40 0"
    "${mainMod} SHIFT, l, resizeactive, 40 0"
    "${mainMod} SHIFT, k, resizeactive, 0 -40"
    "${mainMod} SHIFT, j, resizeactive, 0 40"

    # Scroll
    "${mainMod}, mouse_down, workspace, e+1"
    "${mainMod}, mouse_up, workspace, e-1"

    # Other actions
    "${mainMod}, SPACE, togglefloating"
    "${mainMod}, MINUS, togglesplit"

    "SUPER_SHIFT, W, exec, ${./kill-active.sh}"
    "SUPER_SHIFT, MINUS, pseudo"
    "${mainMod}, F, fullscreen"
  ];

  hyprctlCmd = "${pkgs.hyprland}/bin/hyprctl";

  generic = let
    hyprstfu = inputs.hyprstfu.defaultPackage.${system};
    inherit (inputs.nix-scripts.packages.${system}) sys-sound sys-brightness;
    inherit (pkgs) lib;
  in [
    {
      "mod+key" = "${mainMod}, RETURN";
      "dispatcher" = "exec, ${pkgs.lib.getExe config.programs.ghostty.package}";
    }
    {
      "mod+key" = ",XF86AudioMicMute";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --toggle-mic";
    }
    {
      "mod+key" = ",XF86AudioMute";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --toggle-vol";
    }
    {
      "mod+key" = ",XF86AudioLowerVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --dec";
    }
    {
      "mod+key" = ",XF86AudioRaiseVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --inc";
    }
    {
      "mod+key" = "SUPER, XF86AudioRaiseVolume";
      "dispatcher" = "exec, ${lib.getExe sys-sound} --inc --unleashed";
    }
    {
      "mod+key" = ",XF86MonBrightnessDown";
      "dispatcher" = "exec, ${lib.getExe sys-brightness} --dec";
    }
    {
      "mod+key" = ",XF86MonBrightnessUp";
      "dispatcher" = "exec, ${lib.getExe sys-brightness} --inc";
    }
    {
      "mod+key" = "SUPER_SHIFT, Q";
      "dispatcher" = "exec, ${lib.getExe pkgs.rofi} -show window";
    }
    {
      "mod+key" = "${mainMod}, Q";
      "dispatcher" = "exec, ${lib.getExe pkgs.hyprlauncher}";
    }
    {
      "mod+key" = "SUPER_SHIFT, E";
      "dispatcher" = "exec, ${lib.getExe pkgs.bemoji}";
    }
    {
      "mod+key" = "SUPER_SHIFT, R";
      "dispatcher" = "exec, ${hyprctlCmd} reload";
    }
    {
      # toggle audio of active window
      "mod+key" = "${mainMod}, M";
      "dispatcher" = "exec, ${lib.getExe hyprstfu}";
    }
    {
      "mod+key" = "SUPER_SHIFT, M";
      "dispatcher" = "exec, ${lib.getExe hyprstfu} -unmute-all";
    }
    {
      "mod+key" = "SUPER_SHIFT, XF86AudioLowerVolume";
      "dispatcher" = "exec, ${lib.getExe hyprstfu} -volume 5-";
    }
    {
      "mod+key" = "SUPER_SHIFT, XF86AudioRaiseVolume";
      "dispatcher" = "exec, ${lib.getExe hyprstfu} -volume 5+";
    }
    {
      "mod+key" = "${mainMod}, K";
      "dispatcher" = "exec, ${lib.getExe pkgs-extra.hyprdrop} -i ghostty.hyprdrop 'ghostty --class=ghostty.hyprdrop'";
    }
  ];

  clipboard = let
    inherit (pkgs.lib) getExe;

    grimblastCmd = let
      package = inputs.hyprland-contrib.packages.${system}.grimblast.overrideAttrs (_oldAttrs: {
        prePatch = ''
          substituteInPlace ./grimblast --replace '-t 3000' '-t 3000 -i ${./crop.512.png}'
        '';
      });
    in (pkgs.lib.getExe package);

    rofiCall = name: cmd: ''${getExe pkgs.rofi} -modi "${name}:${cmd}" -show ${name}'';
  in [
    {
      "mod+key" = "SUPER_SHIFT, Print";
      "dispatcher" = "exec, ${grimblastCmd} --freeze --notify copy area";
    }
    {
      "mod+key" = "${mainMod}, Print";
      "dispatcher" = "exec, ${grimblastCmd} --notify copy active";
    }
    {
      # only copy one monitor at a time
      "mod+key" = ",Print";
      "dispatcher" = (
        let
          askTargetCmd = "${getExe pkgs.hyprshot} -m output --clipboard-only";
          allTargetsCmd = "${grimblastCmd} --notify copy screen";
          eval = "${hyprctlCmd} monitors all -j | ${getExe pkgs.jq} \". | length\"";
        in "exec, bash -c 'if [ \"$(${eval})\" -eq 1 ]; then ${allTargetsCmd}; else ${askTargetCmd}; fi'"
      );
    }
    {
      "mod+key" = "SUPER_SHIFT, C";
      "dispatcher" = "exec, ${rofiCall "clipboard" "${getExe inputs.nix-scripts.packages.${system}.cliphist-rofi}"}";
    }
  ];

  playerctl = let
    pctlCmd = pkgs.lib.getExe pkgs.playerctl;
    pctlFallback = cmd: "bash -c 'if ${pctlCmd} --player=spotify status 2>/dev/null | grep -q Playing; then ${pctlCmd} ${cmd} --player=spotify; else ${pctlCmd} ${cmd}; fi'";
  in [
    {
      "mod+key" = "CTRL_SHIFT, braceleft";
      "dispatcher" = "exec, ${pctlFallback "position 5-"}";
    }
    {
      "mod+key" = "CTRL_SHIFT, braceright";
      "dispatcher" = "exec, ${pctlFallback "position 5+"}";
    }
    {
      "mod+key" = "SUPER_SHIFT, braceright";
      "dispatcher" = "exec, ${pctlFallback "next"}";
    }
    {
      "mod+key" = "SUPER_SHIFT, braceleft";
      "dispatcher" = "exec, ${pctlFallback "previous"}";
    }
    {
      "mod+key" = "${mainMod}, Pause";
      "dispatcher" = "exec, ${pctlCmd} play-pause --player=spotify";
    }
    {
      "mod+key" = "${mainMod}, Delete";
      "dispatcher" = "exec, ${pctlCmd} play-pause --all-players --ignore-player=spotify";
    }
  ];
in
  window ++ workspaces ++ builtins.map (b: b."mod+key" + ", " + b.dispatcher) (generic ++ clipboard ++ playerctl)
