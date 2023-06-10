{
  pkgs,
  config,
  ...
}: let
  username = (import ../../owner.nix).username;
in {
  systemd.services.ensure-home-fs = {
    enable = true;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''
        ${pkgs.bash}/bin/bash \
               -c 'mkdir -p /home/${username}/Workspace/playground/ \
                            /home/${username}/Workspace/projects/ \
                            /home/${username}/Saves/ \
                            /home/${username}/Work/ \
                            /home/${username}/Work/ \
                            /home/${username}/Temp/ \
                            /home/${username}/.etc/'
      '';
    };
  };
}
