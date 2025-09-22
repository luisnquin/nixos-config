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
    flake-parts.url = "github:hercules-ci/flake-parts";

    systems.url = "github:nix-systems/default-linux";
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
    flake-compat = {
      type = "github";
      owner = "edolstra";
      repo = "flake-compat";
      flake = false;
    };
    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland = {
      url = "https://github.com/hyprwm/Hyprland";
      inputs = {
        pre-commit-hooks.follows = "";
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
      type = "git";
      submodules = true;
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
    spotify-dbus-control = {
      url = "github:0xc000022070/spotify-dbus-control";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    poetry2nix = {
      url = "github:nix-community/poetry2nix";
      inputs = {
        treefmt-nix.follows = "";
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nix-scripts = {
      url = "github:0xc000022070/nix-scripts";
      inputs = {
        poetry2nix.follows = "poetry2nix";
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    flake-programs-sqlite = {
      url = "github:wamserma/flake-programs-sqlite";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        utils.follows = "flake-utils";
      };
    };
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake/beta";
      inputs = {
        home-manager.follows = "home-manager";
        nixpkgs.follows = "nixpkgs";
      };
    };
    ghostty = {
      url = "github:ghostty-org/ghostty?ref=refs/tags/tip";
      inputs = {
        flake-compat.follows = "flake-compat";
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
    prismlauncher = {
      url = "github:Diegiwg/PrismLauncher-Cracked";
      inputs = {
        flake-compat.follows = "flake-compat";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nixcord = {
      url = "github:kaylorben/nixcord";
      inputs = {
        flake-compat.follows = "flake-compat";
        flake-parts.follows = "flake-parts";
        nixpkgs.follows = "nixpkgs";
      };
    };
    nix-flatpak.url = "github:gmodena/nix-flatpak?ref=latest";
  };

  outputs = inputs @ {
    nixpkgs-extra,
    home-manager,
    nix-nostd,
    hyprland,
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
        overlays =
          import ./overlays/nixpkgs.nix
          ++ [
            (_self: _super: {
              inherit (hyprland.packages.${system}) xdg-desktop-portal-hyprland;
             
              hyprland = hyprland.packages.${system}.hyprland.overrideAttrs (oldAttrs: {
                src = fetchgit {
                  url = "https://github.com/hyprwm/Hyprland";
                  rev = "70a7047ee175d2e7fca1575d50a3738ac40fd2c6";  # Replace with the correct commit or branch
                  sha256 = "1mxbhp6anjiq567prvklnrgv4a0853s1xl0nj27jlkaq66q1j0vl";
                };
              });
            })
          ];

        inherit config system;
      };
    in
      default;

    libx =
      nix-nostd.lib
      // import ./lib {
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
        inputs.flake-programs-sqlite.nixosModules.programs-sqlite
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
        ./home/options
      ];
    };
}
