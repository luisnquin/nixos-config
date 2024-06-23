{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.services.ntfy-sh-client;
in {
  options = {
    services.ntfy-sh-client = {
      enable = lib.mkEnableOption "ntfy-sh-client";

      options = lib.mkOption {
        type = types.attrsOf types.anything;
        description = ''
          Client options to pass to ntfy-sh-client.
        '';
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = with pkgs; [
      ntfy-sh
    ];

    systemd.user.services.ntfy-sh-client = let
      clientConfig = (pkgs.formats.yaml {}).generate "ntfy-sh-client.yaml" cfg.options;
    in {
      Unit = {
        Description = "ntfy-sh client";
      };

      Service = {
        Type = "simple";
        ExecStart = "${pkgs.ntfy-sh}/bin/ntfy sub --config ${clientConfig} --from-config";
        Restart = "on-failure";
        # https://github.com/the-argus/nixsys/blob/521b733b31cf305a2b12cf2022e5079ad9f640d1/user/primary/ntfy.nix#L10
        RestartForceExitStatus = 11;
        RestartSec = 0;

        # disallow writing to /usr, /bin, /sbin, ...
        ProtectSystem = "yes";
        # more paranoid security settings
        NoNewPrivileges = "yes";
        ProtectKernelTunables = "yes";
        ProtectControlGroups = "yes";
        RestrictNamespaces = "yes";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}
