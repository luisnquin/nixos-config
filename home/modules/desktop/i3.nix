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

        exec-nid = "exec --no-startup-id";

        # new-screenshot-path = ''"/home/$USER/Pictures/screenshots/$(${coreutils}/bin/date)"'';
        save-img-stdin-to-clipboard = "${xclip}/bin/xclip -selection clipboard -t image/png";
        capture-active-window = "${maim-path} --window $(${xdotool}/bin/xdotool getactivewindow)";
        capture-selection = "${maim-path} --select";
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
          "Print" = ''${exec-nid} ${maim-path} | ${save-img-stdin-to-clipboard}'';
          "${modifier}+Print" = "${exec-nid} ${capture-active-window} | ${save-img-stdin-to-clipboard}";
          "${modifier}+Shift+Print" = "${exec-nid} ${capture-selection} | ${save-img-stdin-to-clipboard}";
        };

      startup = with pkgs; let
        background-image = ./../../dots/background-image.png;
      in [
        {
          command = "${dex}/bin/dex --autostart --environment i3";
          always = false;
          notification = false;
        }
        {
          command = "${xss-lock}/bin/xss-lock --transfer-sleep-lock -- i3lock --nofork";
          always = false;
          notification = false;
        }
        {
          command = "${networkmanagerapplet}/bin/nm-applet";
          always = false;
          notification = false;
        }
        {
          command = "${nitrogen}/bin/nitrogen --set-auto ${background-image}";
        }
        {
          command = "${numlockx}/bin/numlockx on";
        }
      ];
    };
  };
}
