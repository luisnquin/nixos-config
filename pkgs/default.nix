{
  config,
  pkgs,
  ...
}: let
  tomato = import "/etc/nixos/pkgs/tomato.nix";
in {
  environment.systemPackages = [
    (pkgs.callPackage ./tomato.nix {})
  ];
}
