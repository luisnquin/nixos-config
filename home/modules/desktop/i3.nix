{pkgs, ...}: {
  xsession.windowManager.i3 = {
    enable = true;
    # package = pkgs.i3;

    config = rec {
      modifier = "Mod4";

      window.border = 0;

      gaps = {
        inner = 15;
        outer = 5;
      };

      fonts = {
        names = ["DejaVu Sans Mono" "FontAwesome5Free"];
        style = "Bold Semi-Condensed";
        size = 11.0;
      };

      bars = [
        {
          # I don't know how to reference the file internally created by i3status-rust.nix with xdg.configFile
          statusCommand = "${pkgs.i3status-rust}/bin/i3status-rs ~/.config/i3status-rust/config-bottom.toml";

          colors = {
            separator = "#666666";
            background = "#222222";
            statusline = "#dddddd";

            focusedWorkspace =
              # "#0088CC #0088CC #ffffff";
              {
                background = "#0088CC";
                border = "#0088CC";
                text = "#ffffff";
              };

            activeWorkspace =
              # "#333333 #333333 #ffffff";
              {
                background = "#333333";
                border = "#333333";
                text = "#ffffff";
              };

            inactiveWorkspace =
              # "#333333 #333333 #888888";
              {
                background = "#333333";
                border = "#333333";
                text = "#888888";
              };

            urgentWorkspace =
              #"#2f343a #900000 #ffffff";
              {
                background = "#2f343a";
                border = "#900000";
                text = "#ffffff";
              };
          };
        }
      ];

      keybindings = with pkgs; let
        brightnessctl-path = "${brightnessctl}/bin/brightnessctl";
        amixer-path = "${alsa-utils}/bin/amixer";
        maim-path = "${maim}/bin/maim";
        rofi-path = "${rofi}/bin/rofi";

        # new-screenshot-path = ''"/home/$USER/Pictures/screenshots/$(${coreutils}/bin/date)"'';
        save-img-stdin-to-clipboard = "${xclip}/bin/xclip -selection clipboard -t image/png";
        capture-active-window = "${maim-path} --window $(${xdotool}/bin/xdotool getactivewindow)";
        capture-selection = "${maim-path} --select";
        capture-taken-notification = message: "${pkgs.libnotify}/bin/notify-send 'Screenshot taken!' '${message}' --icon=${./../../dots/icons/screenshot.jpg}";

        exec-nid = "exec --no-startup-id";
      in
        lib.mkOptionDefault {
          "XF86AudioMute" = "exec ${amixer-path} set Master toggle";
          "XF86AudioLowerVolume" = "exec ${amixer-path} set Master 4%-";
          "XF86AudioRaiseVolume" = "exec ${amixer-path} set Master 4%+";
          "XF86MonBrightnessDown" = "exec ${brightnessctl-path} set 4%-";
          "XF86MonBrightnessUp" = "exec ${brightnessctl-path} set 4%+";

          "Ctrl+Shift+e" = "${exec-nid} ${xdg-utils}/bin/xdg-open https://docs.google.com/spreadsheets/u/0/";
          "${modifier}+b" = "exec ${brave}/bin/brave";

          "${modifier}+Shift+minus" = "move scratchpad";
          "${modifier}+minus" = "scratchpad show";
          "${modifier}+q" = "exec ${rofi-path} -modi drun -show drun";
          "${modifier}+Return" = "exec ${alacritty}/bin/alacritty";
          "${modifier}+Shift+q" = "exec ${rofi-path} -show window";
          "${modifier}+Shift+x" = "exec systemctl suspend";

          # Closes the current window
          "${modifier}+Shift+w" = "kill";

          # Screenshots
          "Print" = ''${exec-nid} ${maim-path} | ${save-img-stdin-to-clipboard} && ${capture-taken-notification "From whole screen"}'';
          "${modifier}+Print" = "${exec-nid} ${capture-active-window} | ${save-img-stdin-to-clipboard} && ${capture-taken-notification "From active window"}";
          "${modifier}+Shift+Print" = "${exec-nid} ${capture-selection} | ${save-img-stdin-to-clipboard} && ${capture-taken-notification "From selected area"}";
        };

      startup = let
        background-image = ./../../dots/background-image.png;
      in [
        {
          command = "${pkgs.dex}/bin/dex --autostart --environment i3";
          always = false;
          notification = false;
        }
        {
          command = "${pkgs.xss-lock}/bin/xss-lock --transfer-sleep-lock -- i3lock --nofork";
          always = false;
          notification = false;
        }
        {
          command = "${pkgs.networkmanagerapplet}/bin/nm-applet";
          always = false;
          notification = false;
        }
        {
          command = "${pkgs.picom-next}/bin/picom";
          always = true;
          notification = false;
        }
        # {
        #   command = "systemctl --user restart polybar.service";
        #   always = true;
        #   notification = false;
        # }
        {
          command = "${pkgs.nitrogen}/bin/nitrogen --set-auto ${background-image}";
        }
        {
          command = "${pkgs.numlockx}/bin/numlockx on";
        }
      ];
    };
  };
}
