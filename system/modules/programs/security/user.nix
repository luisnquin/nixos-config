{
  config,
  user,
  pkgs,
  lib,
  ...
}: let
  hotlineGate = pkgs.writeShellApplication {
    name = "hotline-gate";
    runtimeInputs = [pkgs.jq];
    text = builtins.readFile ./hotline-gate.sh;
  };
in {
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
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKpV17zqf4dGsuaddSslVpHV5APCsEQSXPAnuBSZk5zY"
          ''command="${lib.getExe hotlineGate}",restrict,port-forwarding,permitopen="127.0.0.1:5900" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGYReItEQonEGJA7IoQggok/7NWwERWw/QZ+in8rqkur hotline-emulator''
          ''command="${lib.getExe hotlineGate}",restrict,port-forwarding,permitopen="127.0.0.1:5900" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILPidNFv02W5+F9mlSjIkMLKBef9qVxK+yHoOI/qQ23i hotline''
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
