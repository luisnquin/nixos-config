{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [
    # (pkgs.callPackage ./spotifyd.nix {isUnstable = true;})
  ];
}
