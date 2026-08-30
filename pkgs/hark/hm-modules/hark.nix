{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib) mkEnableOption mkIf mkOption types;
  cfg = config.services.hark;

  matchOption = field:
    mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Rust regex the notification's ${field} has to match.";
    };

  alternative = {
    options = {
      appName = matchOption "app name";
      summary = matchOption "summary";
      body = matchOption "body";
      category = matchOption "category";
      desktopEntry = matchOption "desktop entry";
      tag = matchOption "replacement tag";
      urgency = matchOption "urgency";
    };
  };

  group = {name, ...}: {
    options = {
      label = mkOption {
        type = types.str;
        default = name;
        description = "Heading the group is listed under.";
      };

      icon = mkOption {
        type = types.str;
        default = "";
        description = "Glyph beside the heading. Empty falls back to a bell.";
      };

      priority = mkOption {
        type = types.int;
        default = 50;
        description = ''
          Order the rules are tried in, lowest first. A notification joins the
          first group that claims it, so a narrow rule needs a lower number
          than the broad one it would otherwise disappear into.
        '';
      };

      alwaysCollapse = mkOption {
        type = types.bool;
        default = false;
        description = ''
          Keep the group folded however few notifications it holds, instead of
          letting `expandBelow` open it.
        '';
      };

      match = mkOption {
        type = types.listOf (types.submodule alternative);
        default = [];
        description = ''
          Alternatives the group claims a notification on. Every field set
          within one alternative has to match; matching any one alternative is
          enough, which is how an app that announces itself under more than one
          name still lands in a single group.
        '';
      };
    };
  };

  stripUnset = alternatives: map (lib.filterAttrs (_: pattern: pattern != null)) alternatives;

  settings = {
    inherit (cfg) groupRestByApp expandBelow previewChars bodyLines;

    ignore = stripUnset cfg.ignore;

    groups =
      lib.mapAttrs (_: definition: {
        inherit (definition) label icon priority alwaysCollapse;
        match = stripUnset definition.match;
      })
      cfg.groups;
  };
in {
  options.services.hark = {
    enable = mkEnableOption "notification centre over mako's history";

    groups = mkOption {
      type = types.attrsOf (types.submodule group);
      default = {};
      description = ''
        Notifications that must land together whatever their app calls itself.
        Groups are tried in `priority` order and a notification joins the first
        one that claims it.
      '';
      example = lib.literalExpression ''
        {
          agents = {
            label = "Coding agents";
            match = [
              {appName = "(?i)^(claude|codex)";}
              {summary = "(?i)\\b(claude|codex)\\b";}
            ];
          };
        }
      '';
    };

    ignore = mkOption {
      type = types.listOf (types.submodule alternative);
      default = [];
      description = ''
        Notifications the centre never counts, groups or lists. Same shape as a
        group's `match`. An on-screen readout is still a notification, so the
        daemon is the wrong place to drop it; it just has no life after the
        popup.
      '';
      example = lib.literalExpression ''
        [{appName = "(?i)^dunstify$";}]
      '';
    };

    groupRestByApp = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Whether notifications no rule claimed fall into one group per app.
        Off, they share a single catch-all group.
      '';
    };

    expandBelow = mkOption {
      type = types.int;
      default = 4;
      description = ''
        Group size at which the centre stops unfolding a group on its own. A
        short group is worth reading at a glance; a long one is a pile its
        header already summarises.
      '';
    };

    previewChars = mkOption {
      type = types.int;
      default = 160;
      description = "Where a one-line preview is cut.";
    };

    bodyLines = mkOption {
      type = types.int;
      default = 3;
      description = "Lines of body an unfolded notification shows.";
    };
  };

  config = mkIf cfg.enable {
    assertions =
      lib.mapAttrsToList (name: definition: {
        assertion = definition.match != [];
        message = "services.hark.groups.${name} declares no match alternative, so it would never claim a notification.";
      })
      cfg.groups;

    home.packages = [pkgs.hark];

    xdg.configFile."hark/config.json".source =
      (pkgs.formats.json {}).generate "hark.json" settings;
  };
}
