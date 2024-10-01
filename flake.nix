{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # https://github.com/NixOS/nix/issues/3966
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    # it's only beta for my computer! :)
    nixpkgs-beta.url = "nixpkgs/nixos-unstable";
    nixpkgs-extra = {
      url = "github:luisnquin/nixpkgs-extra";
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

    rofi-network-manager = {
      url = "github:luisnquin/rofi-network-manager";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
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
    tplr = {
      url = "github:luisnquin/tplr";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
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
    passgen = {
      url = "github:luisnquin/passgen";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
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
      inputs = {
        nixpkgs.follows = "nixpkgs"; # ! a new Hyprland release will need to follow nixpkgs-beta or to have an updated nixpkgs
        systems.follows = "systems";
      };
      # That process was cancelled because current version is crashing and the logs are not obvious so I am... ashamed...?
      type = "git";
      submodules = true;
    };
    agenix = {
      url = "github:ryantm/agenix";
      inputs = {
        home-manager.follows = "home-manager";
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    spotify-dbus-control = {
      url = "github:luisnquin/spotify-dbus-control";
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
      url = "github:luisnquin/nix-scripts";
      inputs = {
        poetry2nix.follows = "poetry2nix";
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };
    zen-browser = {
      url = "github:MarceColl/zen-browser-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    with inputs; let
      system = "x86_64-linux";

      pkgs = let
        default = import nixpkgs {
          overlays =
            import ./overlays/nixpkgs.nix
            ++ [
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

        unstable-beta = import nixpkgs-beta {
          overlays = import ./overlays/nixpkgs-beta.nix;
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
          inherit extra libx unstable-beta;
        };

      inherit (pkgs) lib;

      metadata = pkgs.libx.mkMetadata ./flake.toml "luisnquin@nyx";

      specialArgs = let
        eval = let
          inherit (metadata.host) desktop;
        in {
          isTiling = builtins.elem desktop ["hyprland" "i3"];
          isWayland = desktop == "hyprland";
        };

        flakes = {};

        packages =
          {
            spicetify = spicetify-nix.legacyPackages.${pkgs.system};
            grub-pkgs = grub-themes.packages.${system};
            zen-browser = zen-browser.packages.${system}.default;

            inherit pkgs;
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
      pkgs.libx.mkSetup {
        inherit (metadata) user host nix;
        inherit pkgs specialArgs;

        flakes = {inherit nixpkgs home-manager;};
        profilesPath = ./home/profiles;
        hostsPath = ./system/hosts;

        nixosModules = [
          (import ./secrets {inherit system agenix;})
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
