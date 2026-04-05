{
  lib,
  config,
  ...
}: let
  cfg = config.programs.opencode;
in {
  options.programs.opencode.plugins = lib.mkOption {
    type =
      lib.types.either
      (lib.types.attrsOf (lib.types.either lib.types.lines lib.types.path))
      lib.types.path;
    default = {};
    description = ''
      Custom plugins for opencode.

      This option can either be:
      - An attribute set defining plugins
      - A path to a directory containing multiple plugin files

      If an attribute set is used, the attribute name becomes the plugin filename,
      and the value is either:
      - Inline content as a string (creates `opencode/plugins/<name>.ts`)
      - A path to a file (creates `opencode/plugins/<name>.ts` or `.js`)

      If a path is used, it is expected to contain plugin files.
      The directory is symlinked to `$XDG_CONFIG_HOME/opencode/plugins/`.
    '';
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = !lib.isPath cfg.plugins || lib.pathIsDirectory cfg.plugins;
        message = "`programs.opencode.plugins` must be a directory when set to a path";
      }
    ];

    xdg.configFile =
      lib.optionalAttrs (lib.isPath cfg.plugins) {
        "opencode/plugins" = {
          source = cfg.plugins;
          recursive = true;
        };
      }
      // lib.optionalAttrs (builtins.isAttrs cfg.plugins) (
        lib.mapAttrs' (
          name: content:
            lib.nameValuePair "opencode/plugins/${name}.ts" (
              if lib.isPath content
              then {source = content;}
              else {text = content;}
            )
        )
        cfg.plugins
      );
  };
}
