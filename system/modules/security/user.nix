{
  user,
  pkgs,
  libx,
  ...
}: {
  users = {
    motd = libx.base64.decode "Q29uZnJvbnQgW2hpXXN0b3J5";
    defaultUserShell = pkgs.zsh;

    users.${user.alias} = {
      description = ''Dorian 🍂'';

      home = ''/home/${user.alias}/'';
      hashedPassword = null;
      isNormalUser = true;

      extraGroups = [
        "networkmanager"
        "docker"
        "wheel"
      ];

      shell = pkgs.zsh;
    };
  };
}
