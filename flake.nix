{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # welcome to my hell ;]
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    nixpkgs-extra = {
      url = "github:0xc000022070/nixpkgs-extra";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };

    # we have this like it couldn't be exposed via builtins.systems
    systems.url = "github:nix-systems/default";
    nix-nostd.url = "github:chessai/nix-std";

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
        flake-utils.follows = "flake-utils";
      };
    };
    hyprstfu = {
      url = "github:luisnquin/hyprstfu";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    senv = {
      url = "github:luisnquin/senv";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
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
        flake-utils.follows = "flake-utils";
      };
    };
    zeroxgen = {
      url = "github:0xc000022070/0xgen";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-scripts = {
      url = "github:0xc000022070/nix-scripts";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
        flake-utils.follows = "flake-utils";
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
      # (FlameFlag) until https://github.com/FlameFlag/nixcord/pull/180 is merged
      url = "github:bddvlpr/nixcord";
      inputs = {
        flake-compat.follows = "";
        nixpkgs.follows = "nixpkgs";
      };
    };
    ghostty = {
      url = "github:ghostty-org/ghostty/tip";
      inputs = {
        flake-compat.follows = "";
        home-manager.follows = "home-manager";
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    # Am I a sinner for having this crap in my PURE system?
    nix-flatpak.url = "github:gmodena/nix-flatpak?ref=latest";
    "3mf2stl" = {
      url = "github:0xc000118128/3mf2stl";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ttree = {
      url = "github:luisnquin/ttree";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    nixpkgs-extra,
    home-manager,
    nixpkgs,
    ...
  }: let
    system = "x86_64-linux";

    pkgs = let
      config = {
        allowBroken = false;
        allowUnfree = true;
      };

      default = import nixpkgs {
        overlays = import ./overlays/nixpkgs.nix {
          inherit inputs system;
        };

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
      pkgs-extra = nixpkgs-extra.packages.${system};

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
        inputs.nao.homeManagerModules.default
        inputs.nixcord.homeModules.nixcord
        inputs.encore.homeModules.default
        inputs.black-terminal.homeModules.default
        ./home/options
      ];
    };
}
