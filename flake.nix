{
  description = "My NixOS configuration";

  inputs = {
    # Specify the source of Home Manager and Nixpkgs.
    nixpkgs.url = "nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "github:the-argus/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    spicetify-nix,
    home-manager,
    ...
  } @ inputs: let
    pkgs = import nixpkgs {
      inherit system;
      config = {allowUnfree = true;};
    };

    metadata = builtins.fromTOML (builtins.readFile ./flake.toml);
    system = "x86_64-linux";

    setSpecialArgs = rec {
      owner = metadata.owner.name;
      user = metadata.users.${metadata.owner.name} // {alias = owner;};
      host = metadata.hosts.${metadata.owner.name};

      inherit spicetify-nix;
    };

    inherit (nixpkgs) lib;
  in {
    homeConfigurations = {
      inherit system;

      luisnquin = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        extraSpecialArgs = setSpecialArgs;

        modules = [
          ./home/home.nix
          ./home/modules/default.nix
        ];
      };
    };

    nixosConfigurations.nyx = lib.nixosSystem {
      inherit system;

      specialArgs = setSpecialArgs;

      modules = [
        ./system/configuration.nix
      ];
    };
  };
}
