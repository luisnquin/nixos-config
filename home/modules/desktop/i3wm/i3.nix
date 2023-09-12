{pkgs, ...}: {
  # https://i3wm.org/docs/userguide.html
  # https://mipmip.github.io/home-manager-option-search/?query=xsession.windowManager.i3.config
  xsession.windowManager.i3 = {
    enable = true;

    config = let
      fonts = {
        names = ["Jetbrains Mono"];
        style = "Bold Semi-Condensed";
        size = 10.0;
      };
    in rec {
      modifier = "Mod4";

      window = {
        border = 0;
        titlebar = false;
      };

      gaps = {
        inner = 15;
        outer = 5;
      };

      terminal = "alacritty";

      inherit fonts;

      bars = [
        {
          # I don't know how to reference the file internally created by i3status-rust.nix with xdg.configFile
          statusCommand = "${pkgs.i3status-rust}/bin/i3status-rs ~/.config/i3status-rust/config-bottom.toml";
          command = "${pkgs.i3}/bin/i3bar -t";
          position = "top";

          inherit fonts;

          trayOutput = "none"; # application icons in status bar

          colors = {
            separator = "#666666";
            background = "#00000000"; # Transparent
            statusline = "#dddddd";

            focusedWorkspace =
              # "#0088CC #0088CC #ffffff";
              {
                background = "#7317cf";
                border = "#7317cf";
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
        inherit (pkgs) callPackage;

        dunstify-sound-bin = "${callPackage ./../../../scripts/dunstify-sound {barColor = "#ffadbb";}}/bin/dunstify-sound";
        dunstify-brightness-bin = "${callPackage ./../../../scripts/dunstify-brightness {}}/bin/dunstify-brightness";
        screen-capture-bin = "${callPackage ./../../../scripts/screen-capture {}}/bin/screen-capture";
        spotify-dbus = "${callPackage ./../../../scripts/spotify-dbus {}}/bin/spotify-dbus";

        rofi-path = "${rofi}/bin/rofi";

        exec-nid = "exec --no-startup-id";
      in
        lib.mkOptionDefault {
          "XF86AudioMicMute" = "${exec-nid} ${dunstify-sound-bin} --toggle-mic";
          "XF86AudioMute" = "${exec-nid} ${dunstify-sound-bin} --toggle-vol";
          "XF86AudioLowerVolume" = "${exec-nid} ${dunstify-sound-bin} --dec";
          "XF86AudioRaiseVolume" = "${exec-nid} ${dunstify-sound-bin} --inc";

          "XF86MonBrightnessDown" = "${exec-nid} ${dunstify-brightness-bin} --dec";
          "XF86MonBrightnessUp" = "${exec-nid} ${dunstify-brightness-bin} --inc";

          "${modifier}+Shift+braceright" = "${exec-nid} ${spotify-dbus} --next";
          "${modifier}+Shift+braceleft" = "${exec-nid} ${spotify-dbus} --prev";
          "${modifier}+Pause" = "${exec-nid} ${spotify-dbus} --toggle";

          "Ctrl+Shift+e" = "${exec-nid} ${xdg-utils}/bin/xdg-open https://docs.google.com/spreadsheets/u/0/";
          "${modifier}+b" = "exec ${brave}/bin/brave";

          "${modifier}+Shift+q" = "exec ${rofi-path} -show window";
          "${modifier}+q" = "exec ${rofi-path} -modi drun -show drun";
          "${modifier}+Shift+c" = ''exec ${rofi-path} -modi "clipboard:greenclip print" -show clipboard'';

          "${modifier}+Shift+minus" = "move scratchpad";
          "${modifier}+minus" = "scratchpad show";
          "${modifier}+Return" = "exec ${alacritty}/bin/alacritty";
          "${modifier}+Shift+x" = "exec systemctl suspend";

          # Closes the current window
          "${modifier}+Shift+w" = "kill";

          # Screenshots
          "Print" = ''${exec-nid} ${screen-capture-bin} --screen'';
          "${modifier}+Print" = "${exec-nid} ${screen-capture-bin} --active-window";
          "${modifier}+Shift+Print" = "${exec-nid} ${screen-capture-bin} --selection";
        };

      startup = let
        background-image = ./../../../dots/background-image.png;
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
          command = "${pkgs.volnoti}/bin/volnoti";
          always = true;
        }
        {
          command = "${pkgs.picom-next}/bin/picom";
          always = true;
          notification = false;
        }
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
