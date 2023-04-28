{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    pkgs.nushell
  ];
}
