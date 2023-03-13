{
  description = "A very basic flake";

  inputs = {
    # Specify the source of Home Manager and Nixpkgs.
    nixpkgs.url = "nixpkgs/nixos-unstable";

    home-manager = {
      url = github:nix-community/home-manager;
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "github:the-argus/spicetify-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    spicetify-nix,
    home-manager,
    ...
  } @ inputs: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config = {allowUnfree = true;};
    };
    lib = nixpkgs.lib;
  in {
    # packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;
    # packages.x86_64-linux.default = self.packages.x86_64-linux.hello;

    #  nixosModules.home = import ./home/home.nix;

    homeConfigurations = {
      inherit system;

      #nyx = {
      #  activationPackage = pkgs.somePackage;
      #  # other configuration options here
      #};

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

    #  homeManagerModules = import ./home.nix;

    nixosConfigurations.nyx = lib.nixosSystem {
      inherit system; # specialArgs = attrs;

      modules = [
        ./system/configuration.nix

        # {
        #   home-manager.useGlobalPkgs = true;
        #   home-manager.useUserPackages = true;
        #   home-manager.users.luisnquin = ./home.nix;
        #   # home-manager.extraSpecialArgs = [spicetify-nix.homeManagerModule ];
        # }
      ];
    };

    #homeConfigurations.luisnquin = home-manager.lib.homeManagerConfiguration {
    #  inherit pkgs;
    #
    #  modules = [
    #    ./home.nix
    #  ];
    #};
  };
}
