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
    nix-cachyos-kernel = {
      url = "github:xddxdd/nix-cachyos-kernel/release";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-parts.follows = "flake-parts";
        flake-compat.follows = "";
      };
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
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake/feat/joined-tabs-folder-session";
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
        flake-parts.follows = "flake-parts";
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
    llm-agents = {
      url = "github:numtide/llm-agents.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
        flake-parts.follows = "flake-parts";
        bun2nix.follows = "";
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
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils"; # TODO: ask them to remove this ponpon shit
      };
    };
    flake-programs-sqlite = {
      url = "github:wamserma/flake-programs-sqlite";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        utils.follows = "flake-utils";
      };
    };
    nix-flatpak.url = "github:gmodena/nix-flatpak?ref=latest";
    "3mf2stl" = {
      url = "github:0xc000118128/3mf2stl";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-substituters = [
      "https://cache.numtide.com"
    ];
    extra-trusted-public-keys = [
      "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
    ];
  };

  outputs = inputs @ {
    nix-cachyos-kernel,
    agentic-flake,
    hyprdysmorphic,
    nixpkgs-extra,
    home-manager,
    llm-agents,
    nixpkgs,
    ...
  }: let
    defaultSystem = "x86_64-linux";
    systems = [defaultSystem];

    mkPkgs = system: let
      config = {
        allowBroken = false;
        allowUnfree = true;
      };

      pkgs = import nixpkgs {
        overlays =
          (import ./overlays/nixpkgs.nix {
            inherit inputs system;
          })
          ++ [
            nix-cachyos-kernel.overlays.default
            hyprdysmorphic.overlays.default
            nixpkgs-extra.overlays.default
            agentic-flake.overlays.default
            llm-agents.overlays.default
          ];

        inherit config system;
      };
    in
      pkgs;

    pkgs = mkPkgs defaultSystem;

    libx = import ./lib {
      inherit pkgs;
    };

    metadata = libx.mkMetadata ./flake.toml "luisnquin@nyx";

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
      inputs.home-manager.nixosModules.default
      (import ./secrets {
        inherit (inputs) agenix;
        system = defaultSystem;
      })
      inputs.agenix.nixosModules.default
      (./system/hosts + "/${metadata.host.name}")
    ];

    homeModules = [
      inputs.battery-notifier.homeManagerModule.default
      inputs.black-terminal.homeModules.default
      inputs.agentic-flake.homeModules.default
      inputs.nao.homeManagerModules.default
      inputs."3mf2stl".homeModules.default
      inputs.nixcord.homeModules.nixcord
      inputs.encore.homeModules.default
      ./home/options
      (./home/profiles + "/${metadata.user.alias}")
    ];
  in
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      inherit systems;

      perSystem = {system, ...}: let
        pkgs = mkPkgs system;
      in {
        packages.setup = pkgs.callPackage ./installer {};
      };

      flake = rec {
        nixosConfigurations."${metadata.host.name}" = nixpkgs.lib.nixosSystem {
          inherit pkgs;
          system = defaultSystem;

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
