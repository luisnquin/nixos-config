{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable"; # https://github.com/NixOS/nix/issues/3966
    home-manager.url = "github:nix-community/home-manager";

    rofi-network-manager.url = "github:luisnquin/rofi-network-manager";
    grub-themes.url = "github:luisnquin/grub-themes";
    scripts.url = "github:luisnquin/scripts";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";

    spicetify-nix.url = "github:the-argus/spicetify-nix";
    hyprland-contrib.url = "github:hyprwm/contrib";
    hyprland.url = "github:hyprwm/Hyprland";
  };

  outputs = inputs:
    with inputs; let
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

      libx = import ./lib {
        inherit (pkgs) lib;
        inherit pkgs;
      };

      metadata = libx.mkMetadata ./flake.toml;

      specialArgs = let
        args = let
          desktopIncluded = list: builtins.elem metadata.host.desktop list;
        in
          {
            pkgsx = import ./pkgs {inherit pkgs;};

            isWayland = desktopIncluded ["hyprland" "sway"];
            isTiling = desktopIncluded ["hyprland" "sway" "i3"];

            grub-pkgs = grub-themes.packages.${system};

            inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            inherit spicetify-nix;
            inherit libx;
          }
          // builtins.mapAttrs (_n: p: p.defaultPackage.${system}) {
            inherit rofi-network-manager senv nao;
          }
          // hyprland-contrib.packages.${system}
          // scripts.packages.${system};
      in
        args
        // import ./overlays/special-args.nix args;
    in
      libx.mkSetup {
        inherit (metadata) user host nix;
        inherit specialArgs pkgs;

        sources = {
          inherit nixpkgs home-manager;
        };

        nixosModules = [
          ./tools/nixos-options
          ./system/configuration.nix
        ];

        homeModules = [
          scripts.homeManagerModules.default
          ./home/options
          ./home/home.nix
        ];
      };
}
