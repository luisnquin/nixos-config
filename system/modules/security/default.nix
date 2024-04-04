{
  imports = [
    ./protocols.nix
    ./groups.nix
    ./user.nix
    ./sudo.nix
  ];

  services.gnome.gnome-keyring.enable = true;
  security.polkit.enable = true;
}
