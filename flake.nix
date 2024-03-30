{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # https://github.com/NixOS/nix/issues/3966
  inputs = {
    # `nixpkgs.url` but even more unstable, use this source when we need to use some specific
    # packages in their latest version to have a feature and/or hotfix ASAP
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
    neovim-flake.url = "github:jordanisaacs/neovim-flake";
    spicetify-nix.url = "github:the-argus/spicetify-nix";
    inner-static.url = "github:luisnquin/inner-static";
    grub-themes.url = "github:luisnquin/grub-themes";
    hyprland-contrib.url = "github:hyprwm/contrib";
    hyprstfu.url = "github:luisnquin/hyprstfu";
    scripts.url = "github:luisnquin/scripts";
    hyprland.url = "github:hyprwm/Hyprland";
    agenix = {
      url = "github:ryantm/agenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    tplr.url = "github:luisnquin/tplr";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";
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

      inherit (pkgs) lib;

      libx =
        (nix-nostd.lib)
        // import ./lib {
          inherit pkgs lib;
        };

      metadata = libx.mkMetadata ./flake.toml "luisnquin@nyx";

      specialArgs = let
        args = let
          desktopIncluded = list: builtins.elem metadata.host.desktop list;
        in
          {
            pkgsx = import ./pkgs {inherit pkgs;} // nixpkgs-extra.packages.${system};

            isWayland = desktopIncluded ["hyprland" "sway"];
            isTiling = desktopIncluded ["hyprland" "sway" "i3"];

            spicetify = spicetify-nix.packages.${pkgs.system}.default;
            mysql_57 = (import nixpkgs_mysql_57 {inherit system;}).mysql57;
            grub-pkgs = grub-themes.packages.${system};

            inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            inherit nixtheplanet neovim-flake libx pkgs;
          }
          // builtins.mapAttrs (_n: p: p.defaultPackage.${system}) {
            inherit rofi-network-manager senv hyprstfu inner-static;
          }
          // hyprland-contrib.packages.${system}
          // scripts.packages.${system};
      in
        args
        // import ./overlays/special-args.nix args;
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
