{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.cliphizt;

  inherit (lib) mkEnableOption mkIf mkOption mkPackageOption types;

  configFile = pkgs.writeText "cliphizt-config" ''
    max-items ${toString cfg.settings.max-items}
    max-dedupe-search ${toString cfg.settings.max-dedupe-search}
    min-store-length ${toString cfg.settings.min-store-length}
    preview-width ${toString cfg.settings.preview-width}
    max-store-size ${cfg.settings.max-store-size}
    ephemeral-ttl ${cfg.settings.ephemeral-ttl}
    persist-mode ${lib.boolToString cfg.settings.persist-mode}
    ${lib.optionalString (cfg.settings.db-path != "") "db-path ${cfg.settings.db-path}"}
  '';
in {
  options.programs.cliphizt = {
    enable = mkEnableOption "cliphizt, a Wayland clipboard history manager";

    package = mkPackageOption pkgs "cliphizt" {};

    settings = mkOption {
      description = "Configuration written to \$XDG_CONFIG_HOME/cliphizt/config.";
      default = {};
      type = types.submodule {
        options = {
          max-items = mkOption {
            type = types.ints.positive;
            default = 750;
            description = "Maximum number of history entries to keep.";
          };

          max-dedupe-search = mkOption {
            type = types.ints.positive;
            default = 100;
            description = "Number of recent entries to scan for duplicates on store.";
          };

          min-store-length = mkOption {
            type = types.ints.unsigned;
            default = 0;
            description = "Minimum codepoint count required to store an entry.";
          };

          preview-width = mkOption {
            type = types.ints.positive;
            default = 100;
            description = ''
              Maximum grapheme cluster width of list preview text. Previews are
              rendered when an entry is stored, so a change here applies to new
              entries; run `cliphizt reindex` to re-render the existing history.
            '';
          };

          max-store-size = mkOption {
            type = types.str;
            default = "5MiB";
            example = "10MiB";
            description = "Maximum byte size of a single entry. Accepts KiB/MiB/GiB suffixes.";
          };

          db-path = mkOption {
            type = types.str;
            default = "";
            example = "/home/user/.local/share/cliphizt/db";
            description = "Override database path. Defaults to \$XDG_CACHE_HOME/cliphizt/db.";
          };

          ephemeral-ttl = mkOption {
            type = types.str;
            default = "1h";
            example = "30m";
            description = ''
              TTL applied to entries stored while in ephemeral mode. Accepts
              s/m/h/d/w units and compound forms like 1h30m.
            '';
          };

          persist-mode = mkOption {
            type = types.bool;
            default = false;
            description = "Persist mode across reboots via \$XDG_STATE_HOME/cliphizt/mode.";
          };
        };
      };
    };

    systemdService = {
      enable = mkEnableOption "systemd user service that watches the clipboard with wl-paste";

      extraStoreArgs = mkOption {
        type = types.listOf types.str;
        default = [];
        example = ["--ttl" "1h"];
        description = "Extra arguments appended to every cliphizt store invocation.";
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile."cliphizt/config".source = configFile;

    systemd.user.services.cliphizt-watch = mkIf cfg.systemdService.enable {
      Unit = {
        Description = "Wayland clipboard history manager (cliphizt)";
        PartOf = ["graphical-session.target"];
        After = ["graphical-session.target"];
      };

      Service = {
        Type = "simple";
        ExecStart = lib.escapeShellArgs (
          ["${pkgs.wl-clipboard}/bin/wl-paste" "--watch" "${lib.getExe cfg.package}" "store"]
          ++ cfg.systemdService.extraStoreArgs
        );
        Restart = "on-failure";
        KillSignal = "SIGINT";
      };

      Install.WantedBy = ["graphical-session.target"];
    };
  };
}
