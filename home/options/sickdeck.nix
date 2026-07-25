{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.sickdeck;
in {
  options.programs.sickdeck = {
    enable = mkEnableOption "the sickdeck client for driving a remote simulator service";

    package = mkPackageOption pkgs "sickdeck" {};

    remote.url = mkOption {
      type = types.str;
      example = "http://rose:4310";
      description = ''
        Service endpoint exported as `SICKDECK_SERVER_URL`. The CLI drives the
        simulators and emulators owned by that service, so none run on this
        host.
      '';
    };

    remote.tokenFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Path to a file holding the service access token. Obsolete in tailnet
        mode, where the service identifies the peer via `tailscale whois` and no
        token exists. The CLI takes a token only through the per-command
        `--access-token` flag, with no env passthrough, so this module cannot
        apply the file for you; leave null unless a token-mode service forces
        it.
      '';
    };

    skill.enable =
      mkEnableOption "installing the generated agent skill to ~/.claude/skills/sickdeck"
      // {
        description = ''
          Install `''${package.skill}/SKILL.md` under `~/.claude/skills`. Leave
          off when another skill manager already publishes it from the same
          output; enabling both writes the same path twice.
        '';
      };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    home.sessionVariables.SICKDECK_SERVER_URL = cfg.remote.url;

    home.file.".claude/skills/sickdeck/SKILL.md" = mkIf cfg.skill.enable {
      source = "${cfg.package.skill}/SKILL.md";
    };
  };
}
