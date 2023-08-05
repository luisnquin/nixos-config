{
  pkgs,
  host,
  ...
}: {
  home.packages = [
    pkgs."${host.browser}"
    pkgs.vivaldi # Always available
  ];
}
