{
  pkgs,
  host,
  ...
}: {
  # https://github.com/greshake/i3status-rust/blob/master/doc/themes.md
  programs.i3status-rust = {
    enable = true;
    package = pkgs.i3status-rust;
    bars = {
      bottom = {
        icons = "material-nf";
        # theme = "space-villain";
        settings = {
          theme = {
            theme = "ctp-mocha";
            overrides = {
              idle_bg = "#3857c7";
              idle_fg = "#abcdef";
            };
          };
        };

        blocks = [
          {
            block = "net";
            format = " ^icon_net_down$speed_down.eng(prefix:K) ^icon_net_up$speed_up.eng(prefix:K) ";
          }
          {
            block = "custom";
            command = "echo '' `curl api.ipify.org`";
            interval = 60;
          }
          {
            block = "docker";
            interval = 7;
            format = "󰡨 $running/$total ";
          }
          {
            block = "sound";
            format = "󰕾 {$volume.eng(w:2) |}";
          }
          # {
          #   block = "github";
          #   format = " $total.eng(w:1)t";
          #   interval = 40;
          #   token = user.gitHubNotificationsToken;
          #   hide_if_total_is_zero = false;
          #   info = ["total"];
          #   warning = ["mention" "review_requested" "security_alert"];
          #
          {
            block = "battery";
            interval = 7;
            full_threshold = host.batteryThreshold;
            format = " $percentage {$time |}";
            full_format = "$icon -> 󱩰 ";
          }
          {
            block = "cpu";
            interval = 5;
            format = " $utilization";
          }
          {
            block = "memory";
            format = "󰍛 $mem_used_percents.eng(w:1)";
            interval = 30;
            warning_mem = 70;
            critical_mem = 85;
          }
          # {
          #   block = "disk_space";
          #   path = "/";
          #   info_type = "available";
          #   format = "󰨣 $available/$total";
          #   interval = 60;
          #   warning = 20.0;
          #   alert = 10.0;
          # }
          {
            block = "nvidia_gpu";
            interval = 1;
            format = "󰢮 $utilization $temperature";
          }
          {
            block = "time";
            interval = 60;
            format = "  $timestamp.datetime(f:'%a %d/%m %R') ";
          }
        ];
      };
    };
  };
}
