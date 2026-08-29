{
  lib,
  pkgs,
  ...
}: {
  environment.systemPackages = [pkgs.sbctl];

  # lanzaboote installs its own copy of systemd-boot; the stock installer
  # would race it for the ESP
  boot.loader.systemd-boot.enable = lib.mkForce false;

  boot.lanzaboote = {
    enable = true;

    # sbctl's key layout; preserved across the root wipe in fs/persistence.nix
    pkiBundle = "/var/lib/sbctl";

    # UKI stubs are tiny but kernels+initrds land on the 500M ESP
    configurationLimit = 10;
  };
}
