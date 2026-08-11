{
  pkgs,
  user,
  ...
}: {
  programs.nix-ld.enable = true; # I'm just a desperate shell

  # previously "programs.adb.enable" but now systemd manages it
  environment.systemPackages = [pkgs.android-tools];

  services.udev.packages = [pkgs.mtkclient];

  users.users.${user.alias}.extraGroups = ["adbusers"];
}
