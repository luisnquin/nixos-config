{
  pkgs,
  host,
  ...
}: {
  environment.systemPackages = [pkgs.xclip];
}
