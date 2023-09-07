{
  pkgs,
  host,
  ...
}:
{
  environment.systemPackages = [pkgs.xclip];
}
// (
  # KDE plasma has its own clipboard manager
  if host.desktop != "plasma"
  then {
    services.greenclip.enable = true;
  }
  else {}
)
