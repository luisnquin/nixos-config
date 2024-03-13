{
  description = "Infrastructure for NixOS, Flakes and home manager ❄️";

  # https://github.com/NixOS/nix/issues/3966
  inputs = {
    home-manager.url = "github:nix-community/home-manager";
    # `nixpkgs.url` but even more unstable, use this source when we need to use some specific
    # packages in their latest version to have a feature and/or hotfix ASAP
    nixpkgs-latest.url = "nixpkgs/nixos-unstable";
    nixpkgs.url = "nixpkgs/nixos-unstable";

    nixpkgs_mysql_57.url = "github:NixOS/nixpkgs?rev=06c9198cbf48559191bf6c9b76c0f370f96b8c33";
    rofi-network-manager.url = "github:luisnquin/rofi-network-manager";
    battery-notifier.url = "github:luisnquin/battery-notifier";
    spicetify-nix.url = "github:the-argus/spicetify-nix";
    grub-themes.url = "github:luisnquin/grub-themes";
    hyprland-contrib.url = "github:hyprwm/contrib";
    hyprstfu.url = "github:luisnquin/hyprstfu";
    scripts.url = "github:luisnquin/scripts";
    hyprland.url = "github:hyprwm/Hyprland";
    tplr.url = "github:luisnquin/tplr";
    senv.url = "github:luisnquin/senv";
    nao.url = "github:luisnquin/nao";
  };

  outputs = inputs:
    with inputs; let
      system = "x86_64-linux";

      mkPkgs = source:
        import source {
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

      pkgs-latest = mkPkgs nixpkgs-latest;
      pkgs = mkPkgs nixpkgs;

      inherit (pkgs) lib;

      libx = import ./lib {
        inherit pkgs lib;
      };

      metadata = libx.mkMetadata ./flake.toml "luisnquin@nyx";

      specialArgs = let
        args = let
          desktopIncluded = list: builtins.elem metadata.host.desktop list;
        in
          {
            pkgsx = import ./pkgs {inherit pkgs;};

            isWayland = desktopIncluded ["hyprland" "sway"];
            isTiling = desktopIncluded ["hyprland" "sway" "i3"];

            spicetify = spicetify-nix.packages.${pkgs.system}.default;
            mysql_57 = (mkPkgs nixpkgs_mysql_57).mysql57;
            grub-pkgs = grub-themes.packages.${system};

            inherit (hyprland.packages.${system}) hyprland xdg-desktop-portal-hyprland;
            inherit libx pkgs pkgs-latest;
          }
          // builtins.mapAttrs (_n: p: p.defaultPackage.${system}) {
            inherit rofi-network-manager senv hyprstfu;
          }
          // hyprland-contrib.packages.${system}
          // scripts.packages.${system};
      in
        args
        // import ./overlays/special-args.nix args;
    in
      libx.mkSetup rec {
        inherit (metadata) user host nix;
        inherit specialArgs pkgs;

        sources = {
          inherit nixpkgs home-manager;
        };

        nixosModules = [
          ./tools/nix/nixos-options
          (./system/hosts + "/${host.name}")
        ];

        homeModules = [
          # scripts.homeManagerModules.default
          battery-notifier.homeManagerModule.default
          spicetify-nix.homeManagerModule
          tplr.homeManagerModules.default
          nao.homeManagerModules.default
          ./home/options
          (./home/profiles + "/${user.alias}")
        ];
      };
}
