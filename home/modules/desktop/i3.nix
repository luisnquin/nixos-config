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
        rofi-path = "${rofi}/bin/rofi";
      in
        lib.mkOptionDefault {
          "XF86AudioMute" = "exec ${amixer-path} set Master toggle";
          "XF86AudioLowerVolume" = "exec ${amixer-path} set Master 4%-";
          "XF86AudioRaiseVolume" = "exec ${amixer-path} set Master 4%+";
          "XF86MonBrightnessDown" = "exec ${brightnessctl-path} set 4%-";
          "XF86MonBrightnessUp" = "exec ${brightnessctl-path} set 4%+";

          # Closes current window
          "${modifier}+Shift+w" = "kill";

          "${modifier}+Shift+minus" = "move scratchpad";
          "${modifier}+minus" = "scratchpad show";
          "${modifier}+q" = "exec ${rofi-path} -modi drun -show drun";
          "${modifier}+Return" = "exec ${alacritty}/bin/alacritty";
          "${modifier}+Shift+q" = "exec ${rofi-path} -show window";
          "${modifier}+b" = "bexec ${brave}/bin/brave";
          "${modifier}+Shift+x" = "exec systemctl suspend";
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
