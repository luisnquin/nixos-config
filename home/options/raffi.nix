{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.raffi;
  yaml = pkgs.formats.yaml {};

  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;

  nullOr = type:
    mkOption {
      inherit type;
      default = null;
    };

  strOrPath = types.oneOf [types.str types.path];

  generalType = types.submodule {
    options = {
      ui_type = nullOr (types.enum ["fuzzel" "native"]);
      default_script_shell = nullOr types.str;
      no_icons = nullOr types.bool;
      max_history = nullOr types.int;
      theme = nullOr types.str;
      font_size = nullOr types.float;
      font_family = nullOr types.str;
      window_width = nullOr types.float;
      window_height = nullOr types.float;
      padding = nullOr types.float;
      sort_mode = nullOr (types.enum ["frequency" "recency" "hybrid"]);

      fallbacks = mkOption {
        type = types.listOf types.str;
        default = [];
      };

      theme_colors = mkOption {
        type = types.attrsOf types.str;
        default = {};
      };
    };
  };

  launcherType = types.submodule {
    options = {
      binary = nullOr types.str;
      script = nullOr strOrPath;

      args = mkOption {
        type = types.listOf types.str;
        default = [];
      };

      description = nullOr types.str;
      icon = nullOr types.str;
      ifexist = nullOr strOrPath;
      ifenvset = nullOr types.str;
      ifenvnotset = nullOr types.str;

      ifenveq = mkOption {
        type = types.nullOr (types.listOf types.str);
        default = null;
        description = ''Example: [ "VAR" "VALUE" ].'';
      };

      disabled = nullOr types.bool;
    };
  };

  scriptFilterType = types.submodule {
    options = {
      name = mkOption {type = types.str;};
      keyword = mkOption {type = types.str;};
      command = mkOption {type = types.str;};

      args = mkOption {
        type = types.listOf types.str;
        default = [];
      };

      icon = nullOr types.str;
      min_query_length = nullOr types.int;

      env = mkOption {
        type = types.attrsOf types.str;
        default = {};
      };

      action = nullOr types.str;
      secondary_action = nullOr types.str;
    };
  };

  textSnippetType = types.submodule {
    options = {
      name = mkOption {type = types.str;};
      keyword = mkOption {type = types.str;};

      icon = nullOr types.str;
      action = nullOr types.str;
      secondary_action = nullOr types.str;

      snippets = mkOption {
        type = types.listOf (types.submodule {
          options = {
            name = mkOption {type = types.str;};
            value = mkOption {type = types.str;};
          };
        });
        default = [];
      };

      file = nullOr strOrPath;
      directory = nullOr strOrPath;
      command = nullOr types.str;

      args = mkOption {
        type = types.listOf types.str;
        default = [];
      };
    };
  };

  webSearchType = types.submodule {
    options = {
      name = mkOption {type = types.str;};
      keyword = mkOption {type = types.str;};
      url = mkOption {type = types.str;};
      icon = nullOr types.str;
    };
  };

  addonToggleType = extraOptions:
    types.submodule {
      options =
        {
          enabled = mkOption {
            type = types.bool;
            default = true;
          };
        }
        // extraOptions;
    };
in {
  options.programs.raffi = {
    enable = mkEnableOption "raffi, a YAML-based application launcher";

    package = mkPackageOption pkgs "raffi" {};

    settings = mkOption {
      type = types.submodule {
        options = {
          version = mkOption {
            type = types.int;
            default = 1;
          };

          general = mkOption {
            type = generalType;
            default = {};
          };

          launchers = mkOption {
            type = types.attrsOf launcherType;
            default = {};
          };

          addons = mkOption {
            type = types.submodule {
              options = {
                script_filters = mkOption {
                  type = types.listOf scriptFilterType;
                  default = [];
                };

                text_snippets = mkOption {
                  type = types.listOf textSnippetType;
                  default = [];
                };

                web_searches = mkOption {
                  type = types.listOf webSearchType;
                  default = [];
                };

                calculator = mkOption {
                  type = addonToggleType {};
                  default = {};
                };

                currency = mkOption {
                  type = addonToggleType {
                    currencies = mkOption {
                      type = types.listOf types.str;
                      default = [];
                    };

                    default_currency = nullOr types.str;
                    trigger = nullOr types.str;
                  };
                  default = {};
                };

                emoji = mkOption {
                  type = addonToggleType {
                    action = nullOr types.str;

                    data_files = mkOption {
                      type = types.listOf strOrPath;
                      default = [];
                    };

                    secondary_action = nullOr types.str;
                    trigger = nullOr types.str;
                  };
                  default = {};
                };

                file_browser = mkOption {
                  type = addonToggleType {
                    show_hidden = nullOr types.bool;
                  };
                  default = {};
                };
              };
            };
            default = {};
          };
        };
      };

      default = {
        version = 1;
      };

      description = ''
        Structured raffi configuration rendered to:
        ~/.config/raffi/raffi.yaml
      '';

      example = lib.literalExpression ''
        {
          version = 1;

          general = {
            ui_type = "native";
            theme = "dark";
            sort_mode = "hybrid";
          };

          launchers.firefox = {
            binary = "firefox";
            icon = "firefox";
            description = "Web browser";
          };

          addons.web_searches = [
            {
              name = "DuckDuckGo";
              keyword = "ddg";
              url = "https://duckduckgo.com/?q={query}";
            }
          ];
        }
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."raffi/raffi.yaml".source =
      yaml.generate "raffi.yaml" cfg.settings;
  };
}
