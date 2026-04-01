{
  imports = [
    ./protocols.nix
    ./network.nix
    ./keyring.nix
    ./groups.nix
    ./user.nix
    ./sudo.nix
    ./tools
  ];

  security.polkit.enable = true;
}
