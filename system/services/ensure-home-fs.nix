{
  pkgs,
  user,
  ...
}: {
  systemd.services.ensure-home-fs = {
    enable = true;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''
        ${pkgs.bash}/bin/bash \
               -c 'mkdir -p /home/${user.alias}/Projects/playground/ \
                            /home/${user.alias}/Saves/ \
                            /home/${user.alias}/Work/ \
                            /home/${user.alias}/Work/ \
                            /home/${user.alias}/Temp/ \
                            /home/${user.alias}/.etc/'
      '';
    };
  };
}
