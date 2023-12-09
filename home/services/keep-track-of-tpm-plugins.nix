{
  config,
  pkgsx,
  lib,
  ...
}: {
  systemd.user.services = lib.mkIf config.programs.tmux.enable {
    keep-track-of-tpm-plugins = {
      Unit = {
        Description = "Install and/or update tmux plugins";
      };

      Service = let
        tpmScript = name: "${pkgsx.tpm}/bin/${name}";
      in {
        Type = "simple";
        ExecStart = "${tpmScript "tpm_install_plugins"} && ${tpmScript "tpm_update_plugins"}";
        Restart = "on-abnormal";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}
