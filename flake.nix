{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # welcome to the hell ;]
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    nixpkgs-reunstable.url = "nixpkgs/nixos-unstable";
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

    systems.url = "github:nix-systems/default-linux";
    nix-nostd.url = "github:chessai/nix-std";

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
        flake-compat.follows = "flake-compat";
      };
    };
    flake-compat = {
      type = "github";
      owner = "edolstra";
      repo = "flake-compat";
      flake = false;
    };
    pre-commit-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        nixpkgs-stable.follows = "nixpkgs";
        flake-compat.follows = "flake-compat";
      };
    };
    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland = {
      url = "https://github.com/hyprwm/Hyprland";
      inputs = {
        pre-commit-hooks.follows = "pre-commit-hooks";
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
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    poetry2nix = {
      url = "github:nix-community/poetry2nix";
      inputs = {
        treefmt-nix.follows = "treefmt-nix";
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
    nixpkgs-terraform = {
      url = "github:stackbuilders/nixpkgs-terraform";
      inputs = {
        nixpkgs-1_0.follows = "nixpkgs";
        nixpkgs-1_6.follows = "nixpkgs";
        nixpkgs-1_9.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    zen-browser = {
      url = "github:0xc000022070/zen-browser-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    ghostty.url = "github:ghostty-org/ghostty";
  };

  outputs = inputs: let
    inherit (inputs) nixpkgs-extra home-manager nix-nostd hyprland nixpkgs nixpkgs-reunstable;

    system = "x86_64-linux";

    pkgs = let
      default = import nixpkgs {
        overlays =
          import ./overlays/nixpkgs.nix
          ++ [
            (_self: _super: {
              inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            })
          ]
          ++ [
            (_self: _super: {
              terraform = inputs.nixpkgs-terraform.packages.${system}."1.9";
            })
          ];
        config = {
          allowBroken = false;
          allowUnfree = true;
        };

        inherit system;
      };

      reunstable = import nixpkgs-reunstable {
        config = {
          allowUnfreePredicate = pkg:
            builtins.elem (lib.getName pkg) [
              "android-studio-stable"
            ];
        };
        inherit system;
      };

      extra = nixpkgs-extra.packages.${system};

      libx =
        (nix-nostd.lib)
        // import ./lib {
          inherit pkgs lib;
        };
    in
      default
      // {
        inherit reunstable extra libx;
      };

    inherit (pkgs) lib;

    metadata = pkgs.libx.mkMetadata ./flake.toml "luisnquin@nyx";

    specialArgs = let
      inherit (metadata.host) desktop;
    in {
      isTiling = builtins.elem desktop ["hyprland" "i3"];
      isWayland = desktop == "hyprland";

      ghostty = inputs.ghostty.packages.${system}.default;

      # Child modules should deal with the things they want to take from the inputs.
      inherit inputs pkgs system;
    };
  in
    pkgs.libx.mkSetup {
      inherit (metadata) user host nix;
      inherit pkgs specialArgs;

      flakes = {inherit nixpkgs home-manager;};
      profilesPath = ./home/profiles;
      hostsPath = ./system/hosts;

      nixosModules = [
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
        ./home/options
      ];
    };
}
