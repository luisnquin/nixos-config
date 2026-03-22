{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # welcome to my hell ;]
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    nixpkgs-extra = {
      url = "github:0xc000022070/nixpkgs-extra";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # we have this like it couldn't be exposed via builtins.systems
    systems.url = "github:nix-systems/default";

    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    battery-notifier = {
      url = "github:luisnquin/battery-notifier";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    grub-themes = {
      url = "github:luisnquin/grub-themes";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    hyprstfu = {
      url = "github:luisnquin/hyprstfu";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    senv = {
      url = "github:luisnquin/senv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    black-terminal.url = "github:luisnquin/black-terminal";
    nao = {
      url = "github:luisnquin/nao";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };

    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprdysmorphic = {
      url = "github:0xc000022070/hyprdysmorphic";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    encore = {
      url = "github:encoredev/encore-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    agenix = {
      url = "github:ryantm/agenix";
      inputs = {
        home-manager.follows = "home-manager";
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    passgen = {
      url = "github:0xc000022070/passgen";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    nix-scripts = {
      url = "github:0xc000022070/nix-scripts";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    nix-index-database = {
      url = "github:nix-community/nix-index-database";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake/beta";
      inputs = {
        home-manager.follows = "home-manager";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nixcord = {
      url = "github:FlameFlag/nixcord";
      inputs = {
        flake-compat.follows = "";
        nixpkgs.follows = "nixpkgs";
        nixpkgs-nixcord.follows = "nixpkgs";
      };
    };
    ghostty = {
      url = "github:ghostty-org/ghostty/tip";
      inputs = {
        flake-compat.follows = "";
        home-manager.follows = "home-manager";
        systems.follows = "systems";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nix-flatpak.url = "github:gmodena/nix-flatpak?ref=latest";
    "3mf2stl" = {
      url = "github:0xc000118128/3mf2stl";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    hyprdysmorphic,
    nixpkgs-extra,
    home-manager,
    nix-scripts,
    grub-themes,
    hyprstfu,
    nixpkgs,
    passgen,
    senv,
    ...
  }: let
    system = "x86_64-linux";

    pkgs = let
      config = {
        allowBroken = false;
        allowUnfree = true;
      };

      default = import nixpkgs {
        overlays =
          (import ./overlays/nixpkgs.nix {
            inherit inputs system;
          })
          ++ [
            hyprdysmorphic.overlays.default
            nixpkgs-extra.overlays.default
            grub-themes.overlays.default
            nix-scripts.overlays.default
            hyprstfu.overlays.default
            passgen.overlays.default
            senv.overlays.default
          ];

        inherit config system;
      };
    in
      default;

    libx = import ./lib {
      inherit pkgs;
    };

    metadata = libx.mkMetadata ./flake.toml "luisnquin@nyx";

    specialArgs = {
      isTiling = true;
      isWayland = true;

      inherit inputs system libx;
    };
  in
    libx.mkSetup {
      inherit (metadata) user host nix;
      inherit pkgs specialArgs;

      flakes = {inherit nixpkgs home-manager;};
      profilesPath = ./home/profiles;
      hostsPath = ./system/hosts;

      nixosModules = [
        inputs.disko.nixosModules.default
        inputs.nix-index-database.nixosModules.default
        inputs.black-terminal.nixosModules.default
        inputs.home-manager.nixosModules.default
        (import ./secrets {
          inherit (inputs) agenix;
          inherit system;
        })
        inputs.agenix.nixosModules.default
        ./tools/nix/nixos-options
      ];

      homeModules = [
        inputs.battery-notifier.homeManagerModule.default
        inputs.spicetify-nix.homeManagerModules.default
        inputs.black-terminal.homeModules.default
        inputs.nao.homeManagerModules.default
        inputs."3mf2stl".homeModules.default
        inputs.nixcord.homeModules.nixcord
        inputs.encore.homeModules.default
        ./home/options
      ];
    };
}
