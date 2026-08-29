{inputs, ...}: {
  imports = [
    inputs.nix-flatpak.homeManagerModules.nix-flatpak
  ];

  services.flatpak.enable = true;

  nix.gc = {
    automatic = true;
    dates = ["daily"];
    options = "--delete-older-than 3d";
    persistent = true;
  };
}
