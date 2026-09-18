{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (config.home) homeDirectory;

  iconTheme = "Tela-pink-dark";
in {
  home.packages = [pkgs.tela-icon-theme];

  programs.vicinae = {
    enable = true;
    systemd.enable = true;

    settings = {
      search_files_in_root = false;
      pop_to_root_on_close = true;
      close_on_focus_loss = true;
      escape_key_behavior = "close_window";

      font.normal.family = "Cascadia Code";

      theme = {
        dark = {
          name = "carbon";
          icon_theme = iconTheme;
        };

        light = {
          name = "carbon";
          icon_theme = iconTheme;
        };
      };

      launcher_window.layer_shell = {
        enabled = true;
        # 'exclusive' breaks action-panel popups under Hyprland and suppresses
        # close_on_focus_loss
        keyboard_interactivity = "on_demand";
      };

      # Allowlist: a full $HOME scan indexes ~1M files, nearly all of it caches
      # the built-in excludes don't cover (.dartServer, .pub-cache, agent state).
      providers.files.preferences.indexingPaths = [
        "${homeDirectory}/Projects"
        "${homeDirectory}/.dotfiles"
      ];
    };

    themes.carbon = {
      meta = {
        version = 1;
        name = "Carbon";
        description = "IBM Carbon dark";
        variant = "dark";
        inherits = "vicinae-dark";
      };

      colors = {
        core = {
          background = "#161616";
          foreground = "#ffffff";
          secondary_background = "#262626";
          border = "#525252";
          accent = "#ee5396";
        };

        accents = {
          blue = "#33b1ff";
          cyan = "#3ddbd9";
          green = "#42be65";
          magenta = "#ee5396";
          orange = "#ff832b";
          purple = "#be95ff";
          red = "#fa4d56";
          yellow = "#f1c21b";
        };
      };
    };
  };

  # The node runtime only serves Raycast TypeScript extensions; every provider
  # used here is native.
  systemd.user.services.vicinae.Service.ExecStart =
    lib.mkForce "${lib.getExe' config.programs.vicinae.package "vicinae"} server --no-extension-runtime";
}
