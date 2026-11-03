{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.pile;

  quote = value: ''"${lib.escape [''"'' "\\"] value}"'';
  criteria = rule: lib.concatStringsSep " " (lib.mapAttrsToList (field: value: "${field}=${quote value}") rule);
in {
  options.services.pile = {
    enable = mkEnableOption "waybar pile for notifications kept off screen";

    rules = mkOption {
      type = types.listOf (types.attrsOf types.str);
      default = [];
      description = ''
        mako criteria, one attrset per section. A notification matching any
        rule never pops up: mako hands it to `pile push` and drops it.
      '';
      example = lib.literalExpression ''
        [
          {app-name = "phone";}
          {app-name = "notify-send"; "summary~" = "(claude|codex)";}
        ]
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [pkgs.pile];

    services.mako.settings = lib.listToAttrs (map (rule:
      lib.nameValuePair (criteria rule) {
        invisible = 1;
        on-notify = ''exec ${lib.getExe pkgs.pile} push "$id"'';
      })
    cfg.rules);
  };
}
