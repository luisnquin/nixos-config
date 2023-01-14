{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment.systemPackages = with pkgs; [
    kubernetes
    minikube
    kubectl
    # lens
    k9s
  ];

  home-manager.users."${owner.username}" = {
    xdg.configFile = {
      # I think that large files are more maintainable in this way
      # "k9s/config.yml".text = builtins.readFile ../dots/home/k9s/config.yml;
      "k9s/views.yml".text = builtins.readFile ../dots/home/k9s/views.yml;
      "k9s/skin.yml".text = builtins.readFile ../dots/home/k9s/skin.yml;
    };
  };
}
/*
programs.k9s = {
    enable = true;

    skin = let
      color = {
        base = "#1e1e2e";
        blue = "#89b4fa";
        crust = "#11111b";
        flamingo = "#f2cdcd";
        green = "#a6e3a1";
        lavender = "#b4befe";
        mantle = "#181825";
        maroon = "#eba0ac";
        mauve = "#cba6f7";
        overlay0 = "#6c7086";
        overlay1 = "#7f849c";
        overlay2 = "#9399b2";
        peach = "#fab387";
        pink = "#f5c2e7";
        red = "#f38ba8";
        rosewater = "#f5e0dc";
        sapphire = "#74c7ec";
        sky = "#89dceb";
        subtext0 = "#a6adc8";
        subtext1 = "#bac2de";
        surface0 = "#313244";
        surface1 = "#45475a";
        surface2 = "#585b70";
        teal = "#94e2d5";
        text = "#cdd6f4";
        yellow = "#f9e2af";
      };
    in {
      k9s = {
        body = {
          fgColor = "${color.text}";
          bgColor = "${color.base}";
          logoColor = ''${color.mauve}'';
        };
        prompt = {
          fgColor = "${color.text}";
          bgColor = "${color.mantle}";
          suggestColor = "${color.blue}";
        };
        info = {
          fgColor = "${color.peach}";
          sectionColor = "${color.text}";
        };
        dialog = {
          fgColor = "${color.yellow}";
          bgColor = "${color.overlay2}";
          buttonFgColor = "${color.base}";
          buttonBgColor = "${color.overlay1}";
          buttonFocusFgColor = "${color.base}";
          buttonFocusBgColor = "${color.pink}";
          labelFgColor = "${color.rosewater}";
          fieldFgColor = "${color.text}";
        };
        frame = {
          border = {
            fgColor = "${color.mauve}";
            focusColor = "${color.lavender}";
          };
          menu = {
            fgColor = "${color.text}";
            keyColor = "${color.blue}";
            numKeyColor = "${color.maroon}";
          };
          crumbs = {
            fgColor = "${color.base}";
            bgColor = "${color.maroon}";
            activeColor = "${color.flamingo}";
          };
          status = {
            newColor = "${color.blue}";
            modifyColor = "${color.lavender}";
            addColor = "${color.green}";
            pendingColor = "${color.peach}";
            errorColor = "${color.red}";
            highlightColor = "${color.sky}";
            killColor = "${color.mauve}";
            completedColor = "${color.overlay0}";
          };
          title = {
            fgColor = "${color.teal}";
            bgColor = "${color.base}";
            highlightColor = "${color.pink}";
            counterColor = "${color.yellow}";
            filterColor = "${color.green}";
          };
        };

        views = {
          charts = {
            bgColor = "${color.base}";
            chartBgColor = "${color.base}";
            dialBgColor = "${color.base}";
            defaultDialColors = ["${color.green}" "${color.red}"];
            defaultChartColors = ["${color.green}" "${color.red}"];
            resourceColors = {
              cpu = ["${color.mauve}" "${color.blue}"];
              mem = ["${color.yellow}" "${color.peach}"];
            };
          };

          table = {
            fgColor = "${color.text}";
            bgColor = "${color.base}";
            cursorFgColor = "${color.surface0}";
            cursorBgColor = "${color.surface1}";
            markColor = "${color.rosewater}";
            header = {
              fgColor = "${color.yellow}";
              bgColor = "${color.base}";
              sorterColor = "${color.sky}";
            };
          };

          xray = {
            fgColor = "${color.text}";
            bgColor = "${color.base}";
            cursorColor = "${color.surface1}";
            cursorTextColor = "${color.base}";
            graphicColor = "${color.pink}";
          };

          yaml = {
            keyColor = "${color.blue}";
            colonColor = "${color.subtext0}";
            valueColor = "${color.text}";
          };

          logs = {
            fgColor = "${color.text}";
            bgColor = "${color.base}";
            indicator = {
              fgColor = "${color.lavender}";
              bgColor = "${color.base}";
            };
          };
        };

        help = {
          fgColor = "${color.text}";
          bgColor = "${color.base}";
          sectionColor = "${color.green}";
          keyColor = "${color.blue}";
          numKeyColor = "${color.maroon}";
        };
      };
    };

    settings = {
      k9s = {
        refreshRate = 2;
        maxConnRetry = 10;
        enableMouse = false;
        headless = false;
        crumbsless = false;
        readOnly = false;
        noIcons = false;
        logger = {
          tail = 50;
          buffer = 2000;
          sinceSeconds = 300;
          fullScreenLogs = false;
          textWrap = false;
          showTime = false;
        };
      };
    };
  };
*/

