{
  description = "My NixOS configuration";
  # future.isDeclarative = true; ❄️

  inputs = {
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

    system = "x86_64-linux";

    setSpecialArgs = let
      metadata = builtins.fromTOML (builtins.readFile ./flake.toml);
      flakeTomlError = message: builtins.throw "error in flake.toml: ${message}";
    in rec {
      owner =
        if metadata.owner.name != null && metadata.owner.name != ""
        then metadata.owner.name
        else flakeTomlError "missing 'owner'";

      user =
        if builtins.hasAttr "${owner}" metadata.users
        then metadata.users.${metadata.owner.name} // {alias = owner;}
        else flakeTomlError "missing '${owner}' owner in users collection";

      host =
        if builtins.hasAttr "${owner}" metadata.hosts
        then metadata.hosts.${metadata.owner.name}
        else flakeTomlError "missing '${owner}' owner in hosts collection";

      inherit spicetify-nix;
    };

    specialArgs = setSpecialArgs;

    inherit (nixpkgs) lib;
  in {
    homeConfigurations = {
      inherit system;

      "${specialArgs.user.alias}" = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        extraSpecialArgs = specialArgs;

        modules = [
          ./home/home.nix
          ./home/modules/default.nix
        ];
      };
    };

    nixosConfigurations."${specialArgs.host.name}" = lib.nixosSystem {
      inherit system;

      specialArgs = specialArgs;

      modules = [
        ./system/configuration.nix
      ];
    };
  };
}
