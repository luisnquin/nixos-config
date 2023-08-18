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

    senv.url = "github:luisnquin/senv";
  };

  outputs = {
    nixpkgs,
    home-manager,
    spicetify-nix,
    senv,
    ...
  }: let
    pkgs = import nixpkgs {
      inherit system;
      config = {allowUnfree = true;};
    };

    system = "x86_64-linux";

    setSpecialArgs = let
      metadata = builtins.fromTOML (builtins.readFile ./flake.toml);
      flakeTomlError = message: builtins.throw "error in flake.toml: ${message}";

      selected =
        if metadata.use != null && metadata.use != ""
        then metadata.use
        else flakeTomlError "missing 'use'";

      data = {
        user =
          if builtins.hasAttr "${selected}" metadata.users
          then metadata.users.${selected} // {alias = selected;}
          else flakeTomlError "missing '${selected}' owner in users collection";

        host =
          if builtins.hasAttr "${selected}" metadata.hosts
          then metadata.hosts.${selected}
          else flakeTomlError "missing '${selected}' owner in hosts collection";
      };

      flakes = {
        inherit spicetify-nix;
        senv = senv.defaultPackage.${system};
      };
    in
      flakes // data;

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
        ];
      };
    };

    nixosConfigurations."${specialArgs.host.name}" = lib.nixosSystem {
      inherit specialArgs;
      inherit system;

      modules = [
        ./system/configuration.nix
      ];
    };
  };
}
