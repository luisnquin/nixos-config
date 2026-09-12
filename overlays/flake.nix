{
  description = "The nixpkgs overlays this repo defines";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {nixpkgs, ...}: {
    overlays.default = nixpkgs.lib.composeManyExtensions (
      import ./nixpkgs.nix {
        host.banner = "";
      }
    );
  };
}
