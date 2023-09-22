{
  pkgs,
  host,
  ...
}: {
  home.packages = [
    pkgs."${host.browser}"
    pkgs.mullvad-browser
    pkgs.vivaldi # Always available
  ];
}
