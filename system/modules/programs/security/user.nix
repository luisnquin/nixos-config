{
  config,
  user,
  pkgs,
  libx,
  lib,
  ...
}: {
  users = {
    motd = libx.base64.decode "Q29uZnJvbnQgW2hpXXN0b3J5";
    defaultUserShell = pkgs.zsh;

    users = {
      ${user.alias} = {
        description = ''Ori^'';

        shell = pkgs.zsh;
        home = ''/home/${user.alias}/'';
        initialHashedPassword = "$y$j9T$FSyIWawN7XrwjmaN5LG5B0$hpO2SDerGvBaoYCfPFbrxcn2j3NS8aTgBfcseMS/QiB";
        isNormalUser = true;

        extraGroups = [
          "networkmanager"
          "wireshark"
          "dialout" # https://askubuntu.com/questions/112568/how-do-i-allow-a-non-default-user-to-use-serial-device-ttyusb0
          "wheel"
        ];

        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICOvNB4XZFchiWUCpdXaNcyoyUi9+7SnGCvrRk2CM129"
        ];
      };

      nginx = lib.mkForce {
        group = "nginx";
        isSystemUser = true;
        uid = config.ids.uids.nginx;
      };
    };
  };
}
