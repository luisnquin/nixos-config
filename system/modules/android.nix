{user, ...}: {
  programs = {
    adb.enable = true;
    nix-ld.enable = true; # I'm just a desperate shell
  };

  users.users.${user.alias}.extraGroups = ["adbusers"];
}
