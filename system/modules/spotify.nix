{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    spotify
  ];
}
