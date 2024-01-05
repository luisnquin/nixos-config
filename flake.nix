{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable"; # https://github.com/NixOS/nix/issues/3966
    home-manager.url = "github:nix-community/home-manager";

    rofi-network-manager.url = "github:luisnquin/rofi-network-manager";
    fallout-grub-theme.url = "github:luisnquin/fallout-grub-theme";
    nix-search.url = "github:luisnquin/nix-search";
    scripts.url = "github:luisnquin/scripts";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";

    spicetify-nix.url = "github:the-argus/spicetify-nix";
    tomato-c.url = "github:gabrielzschmitz/Tomato.C";
    hyprland-contrib.url = "github:hyprwm/contrib";
    hyprland.url = "github:hyprwm/Hyprland";
  };

  outputs = inputs:
    with inputs; let
      setup = let
        metadata = builtins.fromTOML (builtins.readFile ./flake.toml);
        flakeTomlError = message: builtins.throw "error in flake.toml: ${message}";

        notEmptyString = x: x != null && x != "";

        selected =
          if notEmptyString metadata.use
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
          if notEmptyString metadata.nix.stateVersion && notEmptyString metadata.nix.channel
          then metadata.nix
          else flakeTomlError "missing one or more attributes of 'nix'";
      };

      system = "x86_64-linux";

      pkgs = import nixpkgs {
        overlays = import ./overlays/nixpkgs.nix;
        config = {
          permittedInsecurePackages = [
            "electron-19.1.9"
          ];
          allowBroken = false;
          allowUnfree = true;
        };

        inherit system;
      };

      specialArgs = let
        getDefault = pkg: pkg.defaultPackage.${system};

        args = let
          desktopIncluded = list: builtins.elem setup.host.desktop list;
        in
          {
            rofi-network-manager = getDefault rofi-network-manager;
            fallout-grub-theme = getDefault fallout-grub-theme;
            nix-search = getDefault nix-search;
            tomato-c = getDefault tomato-c;
            senv = getDefault senv;
            nao = getDefault nao;

            pkgsx = import ./pkgs {inherit pkgs;};
            libx = import ./lib {
              inherit (pkgs) lib;
              inherit pkgs;
            };

            isWayland = desktopIncluded ["hyprland" "sway"];
            isTiling = desktopIncluded ["hyprland" "sway" "i3"];

            inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            inherit spicetify-nix;
          }
          // hyprland-contrib.packages.${system}
          // scripts.packages.${system}
          // setup;
      in
        args
        // import ./overlays/special-args.nix args;

      mkNixos = config:
        nixpkgs.lib.nixosSystem {
          inherit specialArgs system pkgs;
          modules = [config];
        };

      mkHome = config:
        home-manager.lib.homeManagerConfiguration {
          inherit pkgs;
          extraSpecialArgs = specialArgs;
          modules = [
            scripts.homeManagerModules.default
            config
          ];
        };
    in {
      nixosConfigurations."${setup.host.name}" = mkNixos ./system/configuration.nix;

      homeConfigurations."${setup.user.alias}" = mkHome ./home/home.nix;
    };
}
