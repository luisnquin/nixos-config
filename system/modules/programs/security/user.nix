{
  config,
  user,
  pkgs,
  lib,
  ...
}: {
  users = {
    motd = pkgs.libx.base64.decode "Q29uZnJvbnQgW2hpXXN0b3J5";
    defaultUserShell = pkgs.zsh;

    users = {
      ${user.alias} = {
        description = ''Ori^'';

        home = ''/home/${user.alias}/'';
        hashedPassword = null;
        isNormalUser = true;

        extraGroups = [
          "networkmanager"
          "wireshark"
          "dialout" # https://askubuntu.com/questions/112568/how-do-i-allow-a-non-default-user-to-use-serial-device-ttyusb0
          "docker"
          "wheel"
        ];

        shell = pkgs.zsh;
      };

      nginx = lib.mkForce {
        group = "nginx";
        isSystemUser = true;
        uid = config.ids.uids.nginx;
      };
    };
  };
}
