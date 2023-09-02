{
  description = "My NixOS configuration";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable"; # https://github.com/NixOS/nix/issues/3966
    home-manager.url = "github:nix-community/home-manager";
    spicetify-nix.url = "github:the-argus/spicetify-nix";
    tomato-c.url = "github:gabrielzschmitz/Tomato.C";
    senv.url = "github:luisnquin/senv";
    fallout-grub-theme.url = "github:luisnquin/fallout-grub-theme";
  };

  outputs = inputs:
    with inputs; let
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

      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      specialArgs = let
        getDefault = pkg: pkg.defaultPackage.${system};
      in
        {
          fallout-grub-theme = getDefault fallout-grub-theme;
          tomato-c = getDefault tomato-c;
          senv = getDefault senv;

          inherit spicetify-nix;
        }
        // setup;

      mkNixos = config:
        nixpkgs.lib.nixosSystem {
          inherit specialArgs system;
          modules = [config];
        };

      mkHome = config:
        home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          extraSpecialArgs = specialArgs;
          modules = [config];
        };
    in {
      nixosConfigurations."${setup.host.name}" = mkNixos ./system/configuration.nix;

      homeConfigurations."${setup.user.alias}" = mkHome ./home/home.nix;
    };
}
