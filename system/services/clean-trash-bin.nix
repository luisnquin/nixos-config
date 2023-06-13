{
  config,
  pkgs,
  ...
}: let
  username = (import ../../owner.nix).username;
in {
  systemd = {
    services.clean-computer-trash-bin = {
      enable = true;
      description = "Cleans the bin folder";
      serviceConfig = {
        Type = "simple";
        ExecStart = ''${pkgs.bash}/bin/bash -c "rm -rf /home/${username}/.local/share/Trash/files/{*,.*}"'';
      };

      wantedBy = ["timers.target"];
    };

    timers.clean-computer-trash-bin = {
      description = "Timer for the service to clean the bin folder";
      timerConfig = {
        OnBootSec = "1min";
        OnUnitActiveSec = "1w";
        Persistent = true;
      };

      wantedBy = ["timers.target"];
    };
  };
}
