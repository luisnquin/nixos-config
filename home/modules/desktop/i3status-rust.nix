{pkgs, ...}: {
  programs.i3status-rust = {
    enable = true;
    package = pkgs.i3status-rust;
    bars = {
      bottom = {
        icons = "none";
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
