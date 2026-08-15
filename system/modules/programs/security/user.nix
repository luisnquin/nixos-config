{
  config,
  user,
  pkgs,
  lib,
  ...
}: {
  users = {
    defaultUserShell = pkgs.zsh;

    users = {
      ${user.alias} = {
        description = ''Ori^'';

        shell = pkgs.zsh;
        home = ''/home/${user.alias}/'';
        hashedPasswordFile = config.sops.secrets."users/${user.alias}-password-hash".path;
        isNormalUser = true;

        extraGroups = [
          "networkmanager"
          "wireshark"
          "dialout" # https://askubuntu.com/questions/112568/how-do-i-allow-a-non-default-user-to-use-serial-device-ttyusb0
          "wheel"
        ];

        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMNOYm8dmSXKjgaBQDWCnSvcsGyiJILX3Vwejmkm150+"
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKpV17zqf4dGsuaddSslVpHV5APCsEQSXPAnuBSZk5zY moshi"
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
