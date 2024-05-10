{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # https://github.com/NixOS/nix/issues/3966
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs-extra = {
      url = "github:luisnquin/nixpkgs-extra";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nix-nostd.url = "github:chessai/nix-std";

    nixpkgs_mysql_57.url = "github:NixOS/nixpkgs?rev=06c9198cbf48559191bf6c9b76c0f370f96b8c33";
    rofi-network-manager.url = "github:luisnquin/rofi-network-manager";
    battery-notifier.url = "github:luisnquin/battery-notifier";
    nixtheplanet.url = "github:matthewcroughan/nixtheplanet";
    inner-static.url = "github:luisnquin/inner-static";
    grub-themes.url = "github:luisnquin/grub-themes";
    hyprstfu.url = "github:luisnquin/hyprstfu";
    tplr.url = "github:luisnquin/tplr";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";
    neovim-flake = {
      url = "github:jordanisaacs/neovim-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    spicetify-nix = {
      url = "github:the-argus/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland-contrib = {
      url = "github:hyprwm/contrib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hyprland = {
      url = "github:hyprwm/Hyprland";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    agenix = {
      url = "github:ryantm/agenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    scripts = {
      url = "github:luisnquin/scripts";
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
          ];
        config = {
          allowBroken = false;
          allowUnfree = true;
        };

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
            pkgsx = import ./pkgs {inherit pkgs;} // nixpkgs-extra.packages.${system};

            spicetify = spicetify-nix.packages.${pkgs.system}.default;
            mysql_57 = (import nixpkgs_mysql_57 {inherit system;}).mysql57;
            grub-pkgs = grub-themes.packages.${system};

            inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            inherit libx pkgs;
          }
          // builtins.mapAttrs (_n: p: p.defaultPackage.${system}) {
            inherit rofi-network-manager senv hyprstfu inner-static;
          }
          // hyprland-contrib.packages.${system}
          // scripts.packages.${system};
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
          ./system/options
          ./tools/nix/nixos-options
        ];

        homeModules = [
          battery-notifier.homeManagerModule.default
          spicetify-nix.homeManagerModule
          tplr.homeManagerModules.default
          nao.homeManagerModules.default
          ./home/options
        ];
      };
}
