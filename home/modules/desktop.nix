{pkgs, ...}: {
  # https://github.com/Th0rgal/horus-nix-home/blob/master/configs/i3.nix
  # https://github.com/Th0rgal/horus-nix-home/blob/master/configs/compton.nix
  # https://www.reddit.com/r/unixporn/comments/fltmar/i3gaps_nixos_arch_my_incredible_nixos_desktop/
  # https://i3wm.org/docs/userguide.html#configuring
  # https://www.google.com/search?q=i3+enable+transparency&oq=i3+enable+&aqs=chrome.4.69i57j0i19i512l3j0i19i22i30l6.6319j0j1&sourceid=chrome&ie=UTF-8
  # https://github.com/vivien/i3blocks#example
  # https://www.reddit.com/r/NixOS/comments/vdlq9e/how_to_use_window_managers_in_nixos/?rdt=38930
  # https://github.com/i3-wsman/i3-wsman
  programs.rofi.enable = true;

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
        background-img-path = "~/Projects/github.com/luisnquin/walls/desktop/landscapes/i-wanted-a-large-day.jpg";
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
          command = "${nitrogen}/bin/nitrogen --set-auto ${background-img-path}";
        }
        {
          command = "${numlockx}/bin/numlockx on";
        }
      ];
    };
  };

  programs.i3status-rust = {
    enable = true;
    package = pkgs.i3status-rust;
    bars = {
      top = {
        icons = "Symbols Nerd";
        theme = "gruvbox-dark";
        settings = {
          theme = {
            theme = "solarized-dark";
            overrides = {
              idle_bg = "#123456";
              idle_fg = "#abcdef";
            };
          };
        };

        blocks = [
          {
            block = "disk_space";
            path = "/";
            info_type = "available";
            interval = 60;
            warning = 20.0;
            alert = 10.0;
          }
          {
            block = "memory";
            format_mem = " $icon $mem_used_percents ";
            format_swap = " $icon $swap_used_percents ";
          }
          {
            block = "cpu";
            interval = 1;
          }
          {
            block = "load";
            interval = 1;
            format = " $icon $1m ";
          }
          {
            block = "sound";
          }
          {
            block = "time";
            interval = 60;
            format = " $timestamp.datetime(f:'%a %d/%m %R') ";
          }
        ];
      };
    };
  };
}
