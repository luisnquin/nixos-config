{
  description = "My flake";

  inputs = {
    # Specify the source of Home Manager and Nixpkgs.
    nixpkgs.url = "nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "github:the-argus/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hyprland.url = "github:hyprwm/Hyprland";
  };

  outputs = {
    self,
    nixpkgs,
    hyprland,
    spicetify-nix,
    home-manager,
    ...
  } @ inputs: let
    pkgs = import nixpkgs {
      inherit system;
      config = {allowUnfree = true;};
    };

    system = "x86_64-linux";
    inherit (nixpkgs) lib;
  in {
    homeConfigurations = {
      inherit system;

      luisnquin = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        extraSpecialArgs = {
          inherit spicetify-nix;
        };

        modules = [
          ./home/home.nix
          ./home/modules/default.nix
        ];
      };
    };

    nixosConfigurations.nyx = lib.nixosSystem {
      inherit system;

      modules = [
        hyprland.nixosModules.default
        {programs.hyprland.enable = true;}
        ./system/configuration.nix
      ];
    };
  };
}
