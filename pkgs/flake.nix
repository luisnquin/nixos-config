{
  description = "The packages this repo defines, reachable without its host configurations";

  # ./default.nix stays the single definition; the root flake imports it by path.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: let
    systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

    forAllSystems = f:
      nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    overlays.default = final: _prev: import ./default.nix final;

    # The whole set on every system; attributes are lazy, so reading one never
    # evaluates a package the platform lacks.
    packages = forAllSystems (pkgs: import ./default.nix pkgs);
  };
}
