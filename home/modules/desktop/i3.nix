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

          "${modifier}+Shift+minus" = "move scratchpad";
          "${modifier}+minus" = "scratchpad show";
          "${modifier}+q" = "exec ${rofi-path} -modi drun -show drun";
          "${modifier}+Return" = "exec ${alacritty}/bin/alacritty";
          "${modifier}+Shift+q" = "exec ${rofi-path} -show window";
          "${modifier}+b" = "bexec ${brave}/bin/brave";
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
