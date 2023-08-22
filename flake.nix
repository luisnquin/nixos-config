{
  description = "My NixOS configuration";
  # future.isDeclarative = true; ❄️

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable"; # https://github.com/NixOS/nix/issues/3966
    home-manager.url = "github:nix-community/home-manager";
    spicetify-nix.url = "github:the-argus/spicetify-nix";
    tomato-c.url = "github:gabrielzschmitz/Tomato.C";
    senv.url = "github:luisnquin/senv";
  };

  outputs = {
    nixpkgs,
    home-manager,
    spicetify-nix,
    tomato-c,
    senv,
    ...
  }: let
    setup = let
      metadata = builtins.fromTOML (builtins.readFile ./flake.toml);
      flakeTomlError = message: builtins.throw "error in flake.toml: ${message}";

      hasValue = x: x != null && x != "";

      selected =
        if hasValue metadata.use
        then metadata.use
        else flakeTomlError "missing 'use'";
    in {
      user =
        if builtins.hasAttr "${selected}" metadata.users
        then metadata.users.${selected} // {alias = selected;}
        else flakeTomlError "missing '${selected}' owner in users collection";

      host =
        if builtins.hasAttr "${selected}" metadata.hosts
        then metadata.hosts.${selected}
        else flakeTomlError "missing '${selected}' owner in hosts collection";

      nix =
        if hasValue metadata.nix.stateVersion && hasValue metadata.nix.channel
        then metadata.nix
        else flakeTomlError "missing one or more attributes of 'nix'";
    };

    specialArgs =
      {
        tomato-c = tomato-c.defaultPackage.${system};
        senv = senv.defaultPackage.${system};

        inherit spicetify-nix;
      }
      // setup;

    pkgs = import nixpkgs {
      inherit system;
      config = {allowUnfree = true;};
    };

    system = "x86_64-linux";
  in {
    homeConfigurations = {
      inherit system;

      "${specialArgs.user.alias}" = home-manager.lib.homeManagerConfiguration {
        extraSpecialArgs = specialArgs;
        inherit pkgs;

        modules = [
          ./home/home.nix
        ];
      };
    };

    nixosConfigurations."${specialArgs.host.name}" = nixpkgs.lib.nixosSystem {
      inherit specialArgs system;

      modules = [
        ./system/configuration.nix
      ];
    };
  };
}
