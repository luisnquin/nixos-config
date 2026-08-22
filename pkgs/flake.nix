{
  description = "The packages this repo defines, reachable without its host configurations";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: let
    systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

    forAllSystems = f:
      nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    overlays.default = final: _prev: import ./default.nix final;

    homeModules.default = ./hm-modules;
    nixosModules.default = ./nixos-modules;

    packages = forAllSystems (pkgs: import ./default.nix pkgs);
  };
}
