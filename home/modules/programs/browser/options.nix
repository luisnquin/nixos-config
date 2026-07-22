{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;

  cfg = config.programs.browsers;

  jsonFormat = pkgs.formats.json {};

  stripNulls = value:
    if lib.isAttrs value
    then lib.mapAttrs (_: stripNulls) (lib.filterAttrs (_: v: v != null) value)
    else if lib.isList value
    then map stripNulls value
    else value;

  openerType = types.submodule {
    options = {
      profile = mkOption {
        type = types.str;
        description = "Profile id, as `<executable>#<profile>`.";
      };

      incognito = mkOption {
        type = types.bool;
        default = false;
        description = "Whether to open the link in a private window.";
      };
    };
  };
in {
  options.programs.browsers = {
    enable = mkEnableOption "Browsers, a picker for links opened outside a browser";

    package = mkOption {
      type = types.package;
      default = pkgs.browsers;
      defaultText = lib.literalExpression "pkgs.browsers";
      description = "Package providing the browsers binary.";
    };

    settings = mkOption {
      type = types.submodule {
        freeformType = jsonFormat.type;

        options = {
          hidden_apps = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "App ids kept out of the picker.";
          };

          hidden_profiles = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Profile ids kept out of the picker.";
          };

          profile_order = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Profile ids in picker order. Unlisted profiles follow.";
          };

          default_profile = mkOption {
            type = types.nullOr openerType;
            default = null;
            description = "Opener used when no rule matches. Set, the picker never shows.";
          };

          rules = mkOption {
            type = types.listOf (types.submodule {
              options = {
                source_app = mkOption {
                  type = types.nullOr types.str;
                  default = null;
                  description = "App id the link has to come from.";
                };

                url_pattern = mkOption {
                  type = types.nullOr types.str;
                  default = null;
                  description = "Glob over `scheme://host/path?query#fragment`. A bare host matches any of them.";
                };

                opener = mkOption {
                  type = types.nullOr openerType;
                  default = null;
                  description = "Where a matching link goes, bypassing the picker.";
                };
              };
            });
            default = [];
            description = "Rules evaluated in order before the picker opens.";
          };

          ui = mkOption {
            type = types.submodule {
              options = {
                show_hotkeys = mkOption {
                  type = types.bool;
                  default = true;
                  description = "Whether to label each entry with its number key.";
                };

                quit_on_lost_focus = mkOption {
                  type = types.bool;
                  default = false;
                  description = "Whether to close the picker on focus loss. Fires on any popup outside macOS.";
                };

                theme = mkOption {
                  type = types.enum ["Auto" "Light" "Dark"];
                  default = "Auto";
                  description = "Picker theme. `Auto` reads the desktop portal.";
                };
              };
            };
            default = {};
            description = "Picker appearance.";
          };

          behavior = mkOption {
            type = types.submodule {
              options.unwrap_urls = mkOption {
                type = types.bool;
                default = false;
                description = "Whether to resolve outlook safelinks and messenger redirects before matching rules.";
              };
            };
            default = {};
            description = "Link handling.";
          };
        };
      };

      default = {};
      description = "Browsers configuration written to config.json.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."software.Browsers/config.json".source =
      jsonFormat.generate "browsers-config.json" (stripNulls cfg.settings);

    # home.sessionVariables = mkIf cfg.setAsDefaultBrowser {
    #   BROWSER = lib.mkForce cfg.package.meta.mainProgram;
    # };
  };
}
