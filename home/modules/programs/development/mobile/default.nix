{pkgs, ...}: {
  imports = [
    ./options.nix
    ./android.nix
    ./avds.nix
  ];

  home.packages = [pkgs.phone];
}
