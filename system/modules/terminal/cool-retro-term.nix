{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    pkgs.cool-retro-term
  ];
}
