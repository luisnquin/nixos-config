{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # welcome to my hell ;]
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
    systems.url = "github:nix-systems/default";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.systems.follows = "systems";
    };
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

    black-terminal.url = "github:luisnquin/black-terminal";
    hyprdysmorphic = {
      url = "github:0xc000022070/hyprdysmorphic";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    xgreeter = {
      url = "github:0xc000022070/xgreeter";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    encore = {
      url = "github:encoredev/encore-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake/beta"; # 1.130238β
      inputs = {
        home-manager.follows = "home-manager";
        nixpkgs.follows = "nixpkgs";
      };
    };
    clipz = {
      url = "github:luisnquin/clipz";
      inputs.nixpkgs.follows = "nixpkgs";
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
    herdr = {
      url = "github:ogulcancelik/herdr";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
        flake-parts.follows = "flake-parts";
      };
    };
    nao = {
      url = "github:luisnquin/nao";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    agentic-flake = {
      url = "github:0xc000022070/agentic-flake";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        home-manager.follows = "home-manager";
      };
    };
    roborev = {
      url = "github:roborev-dev/roborev";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hermes-agent = {
      url = "github:NousResearch/hermes-agent";
      inputs = {
        flake-parts.follows = "flake-parts";
        nixpkgs.follows = "nixpkgs";
      };
    };
    flake-programs-sqlite = {
      url = "github:wamserma/flake-programs-sqlite";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        utils.follows = "flake-utils";
      };
    };
    techdebt-cli = {
      url = "github:0xc000022070/techdebt-cli";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-flatpak.url = "github:gmodena/nix-flatpak?ref=latest";
    "3mf2stl" = {
      url = "github:0xc000118128/3mf2stl";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    home-manager,
    nixpkgs,
    ...
  }: let
    defaultSystem = "x86_64-linux";
    systems = [defaultSystem];

    # Off nixpkgs.lib, not libx, so overlays can read it without recursing
    # through the pkgs set they are applied to.
    metadata =
      (import ./lib/metadata.nix {inherit (nixpkgs) lib;})
      .mkMetadata ./flake.toml "luisnquin@nyx";

    mkPkgs = system: let
      config = {
        allowBroken = false;
        allowUnfree = true;
        android_sdk.accept_license = true;
      };

      pkgs = import nixpkgs {
        overlays =
          import ./overlays/inputs.nix {
            inherit inputs system;
          }
          ++ import ./overlays/nixpkgs.nix {
            inherit inputs system;
            inherit (metadata) host;
          };

        inherit config;
        localSystem.system = system;
      };
    in
      pkgs;

    pkgs = mkPkgs defaultSystem;

    libx = import ./lib {
      inherit pkgs;
    };

    specialArgs = {
      isTiling = true;
      isWayland = true;

      inherit inputs libx;
      system = defaultSystem;
    };

    configArgs =
      specialArgs
      // {
        inherit (metadata) host user nix;
      };

    nixosModules = [
      inputs.flake-programs-sqlite.nixosModules.programs-sqlite
      inputs.disko.nixosModules.default
      inputs.black-terminal.nixosModules.default
      inputs.xgreeter.nixosModules.default
      inputs.home-manager.nixosModules.default
      inputs.sops-nix.nixosModules.sops
      ./secrets
      (./system/hosts + "/${metadata.host.name}")
    ];

    homeModules = [
      inputs.battery-notifier.homeManagerModule.default
      inputs.black-terminal.homeModules.default
      inputs.agentic-flake.homeModules.default
      inputs.nao.homeManagerModules.default
      inputs.clipz.homeManagerModules.default
      inputs."3mf2stl".homeModules.default
      inputs.encore.homeModules.default
      inputs.techdebt-cli.homeModules.default
      ./home/options
      (./home/profiles + "/${metadata.user.alias}")
    ];
  in
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      inherit systems;

      perSystem = {system, ...}: let
        pkgs = mkPkgs system;
      in {
        packages = import ./pkgs pkgs;
      };

      flake = rec {
        nixosConfigurations."${metadata.host.name}" = nixpkgs.lib.nixosSystem {
          inherit pkgs;

          modules = nixosModules;
          specialArgs =
            configArgs
            // {
              hmConfig = homeConfigurations."${metadata.user.alias}".config;
            };
        };

        homeConfigurations."${metadata.user.alias}" = home-manager.lib.homeManagerConfiguration {
          inherit pkgs;

          modules = homeModules;
          extraSpecialArgs =
            configArgs
            // {
              system = defaultSystem;
              nixosConfig = nixosConfigurations."${metadata.host.name}".config;
            };
        };
      };
    };
}
