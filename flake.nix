{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # https://github.com/NixOS/nix/issues/3966
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    # it's only beta for my computer! :)
    nixpkgs-beta.url = "nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs-extra = {
      url = "github:luisnquin/nixpkgs-extra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-nostd.url = "github:chessai/nix-std";

    rofi-network-manager.url = "github:luisnquin/rofi-network-manager";
    battery-notifier.url = "github:luisnquin/battery-notifier";
    nixtheplanet.url = "github:matthewcroughan/nixtheplanet";
    grub-themes.url = "github:luisnquin/grub-themes";
    hyprstfu.url = "github:luisnquin/hyprstfu";
    tplr.url = "github:luisnquin/tplr";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";
    passgen = {
      url = "github:luisnquin/passgen";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    neovim-flake = {
      url = "github:jordanisaacs/neovim-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland = {
      url = "https://github.com/hyprwm/Hyprland";
      inputs.nixpkgs.follows = "nixpkgs"; # ! a new Hyprland release will need to follow nixpkgs-beta or to have an updated nixpkgs
      # That process was cancelled because current version is crashing and the logs are not obvious so I am... ashamed...?
      type = "git";
      submodules = true;
    };
    agenix = {
      url = "github:ryantm/agenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    spotify-dbus-control = {
      url = "github:luisnquin/spotify-dbus-control";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-scripts = {
      url = "github:luisnquin/nix-scripts";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    templ = {
      url = "github:a-h/templ";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    with inputs; let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        overlays =
          import ./overlays/nixpkgs.nix
          ++ [
            templ.overlays.default
            (_self: _super: {
              inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            })
          ];
        config = {
          allowBroken = false;
          allowUnfree = true;
        };

        inherit system;
      };

      pkgs-beta = import nixpkgs-beta {
        overlays = import ./overlays/nixpkgs-beta.nix;
        inherit system;
      };

      inherit (pkgs) lib;

      libx =
        (nix-nostd.lib)
        // import ./lib {
          inherit pkgs lib;
        };

      metadata = libx.mkMetadata ./flake.toml "luisnquin@nyx";

      specialArgs = let
        eval = let
          inherit (metadata.host) desktop;
        in {
          isTiling = builtins.elem desktop ["hyprland" "i3"];
          isWayland = desktop == "hyprland";
        };

        flakes = {
          inherit nixtheplanet neovim-flake;
        };

        packages =
          {
            pkgsx = nixpkgs-extra.packages.${system};

            spicetify = spicetify-nix.legacyPackages.${pkgs.system};
            grub-pkgs = grub-themes.packages.${system};

            inherit libx pkgs pkgs-beta;
          }
          // builtins.mapAttrs (_n: p: p.defaultPackage.${system}) {
            inherit rofi-network-manager senv passgen hyprstfu spotify-dbus-control;
          }
          // hyprland-contrib.packages.${system}
          // nix-scripts.packages.${system};
      in
        packages
        // flakes
        // eval
        // import ./overlays/special-args.nix packages;
    in
      libx.mkSetup {
        inherit (metadata) user host nix;
        inherit pkgs specialArgs;

        flakes = {inherit nixpkgs home-manager;};
        profilesPath = ./home/profiles;
        hostsPath = ./system/hosts;

        nixosModules = [
          (import ./secrets {inherit system agenix;})
          nixtheplanet.nixosModules.macos-ventura
          agenix.nixosModules.default
          ./tools/nix/nixos-options
        ];

        homeModules = [
          battery-notifier.homeManagerModule.default
          spicetify-nix.homeManagerModules.default
          tplr.homeManagerModules.default
          nao.homeManagerModules.default
          ./home/options
        ];
      };
}
