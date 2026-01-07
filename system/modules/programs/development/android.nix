{
  pkgs,
  user,
  ...
}: {
  programs.nix-ld.enable = true; # I'm just a desperate shell

  # previously "programs.adb.enable" but now systemd manages it
  environment.systemPackages = [pkgs.android-tools];

  users.users.${user.alias}.extraGroups = ["adbusers"];
}
