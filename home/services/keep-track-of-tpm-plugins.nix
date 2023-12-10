{
  config,
  pkgsx,
  pkgs,
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
        Type = "oneshot";
        ExecStart = "${pkgs.bash}/bin/bash -c '${tpmScript "tpm_install_plugins"} && ${tpmScript "tpm_update_plugins"} all'";
        Restart = "on-abnormal";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}
