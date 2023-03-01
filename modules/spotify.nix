{
  config,
  pkgs,
  ...
}
: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment.systemPackages = with pkgs; [
    spotify
  ];
}
