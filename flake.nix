{
  description = "My NixOS configuration";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable"; # https://github.com/NixOS/nix/issues/3966
    home-manager.url = "github:nix-community/home-manager";
    spicetify-nix.url = "github:the-argus/spicetify-nix";
    tomato-c.url = "github:gabrielzschmitz/Tomato.C";
    fallout-grub-theme.url = "github:luisnquin/fallout-grub-theme";
    nix-search.url = "github:luisnquin/nix-search";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";

    hyprland-contrib.url = "github:hyprwm/contrib";
    hyprland.url = "github:hyprwm/Hyprland";
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

        config = {
          permittedInsecurePackages = [
            "electron-12.2.3"
            "nodejs-16.20.2"
          ];
          allowBroken = false;
          allowUnfree = true;
        };
      };

      specialArgs = let
        getDefault = pkg: pkg.defaultPackage.${system};
      in
        {
          inherit (hyprland.packages.${system}) hyprland;
          fallout-grub-theme = getDefault fallout-grub-theme;
          nix-search = getDefault nix-search;
          tomato-c = getDefault tomato-c;
          senv = getDefault senv;
          nao = getDefault nao;

          inherit spicetify-nix;
        }
        // hyprland-contrib.packages.${system}
        // setup;

      mkNixos = config:
        nixpkgs.lib.nixosSystem {
          inherit specialArgs system pkgs;
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
