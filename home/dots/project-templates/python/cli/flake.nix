{
  inputs = {
    nixpkgs.url = "nixpkgs";
    poetry2nix.url = "github:nix-community/poetry2nix";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs = {
    poetry2nix,
    systems,
    nixpkgs,
  }: let
    inherit (nixpkgs) lib;

    pkgsFor = eachSystem (system:
      import nixpkgs {
        localSystem = system;
      });

    eachSystem = lib.genAttrs (import systems);
  in rec {
    packages = eachSystem (
      system: let
        pkgs = pkgsFor.${system};
      in {
        default = pkgs.callPackage ./default.nix {
          inherit (poetry2nix.lib.mkPoetry2Nix {inherit pkgs;}) mkPoetryApplication;
        };
      }
    );
  };
}
