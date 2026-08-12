{pkgs, ...}: {
  imports = [
    ./options.nix
    ./android.nix
    ./avds.nix
    ./sickdeck.nix
  ];

  home.packages = [pkgs.phone];
}
