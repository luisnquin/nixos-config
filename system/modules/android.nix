{user, ...}: {
  programs.adb.enable = true;

  users.users.${user.alias}.extraGroups = ["adbusers"];
}
