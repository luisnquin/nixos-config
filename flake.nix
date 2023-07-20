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

    setSpecialArgs = let
      flakeTomlError = message: "error in flake.toml: ${message}";
    in rec {
      owner =
        if metadata.owner.name != null && metadata.owner.name != ""
        then metadata.owner.name
        else builtins.throw (flakeTomlError "missing 'owner'");

      user =
        if builtins.hasAttr "${owner}" metadata.users
        then metadata.users.${metadata.owner.name} // {alias = owner;}
        else builtins.throw (flakeTomlError "missing '${owner}' owner in users collection");

      host =
        if builtins.hasAttr "${owner}" metadata.hosts
        then metadata.hosts.${metadata.owner.name}
        else builtins.throw (flakeTomlError "missing '${owner}' owner in hosts collection");

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
