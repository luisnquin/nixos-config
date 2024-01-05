let
  mkNixos = {
    nixosModules,
    configArgs,
    hmConfig,
    nixpkgs,
    pkgs,
  }: let
    specialArgs =
      configArgs
      // {
        inherit hmConfig;
      };
  in
    nixpkgs.lib.nixosSystem {
      inherit (pkgs) system;
      inherit specialArgs pkgs;

      modules = nixosModules;
    };

  mkHome = {
    home-manager,
    nixosConfig,
    homeModules,
    configArgs,
    pkgs,
  }: let
    extraSpecialArgs =
      configArgs
      // {
        inherit nixosConfig;
      };
  in
    home-manager.lib.homeManagerConfiguration {
      inherit pkgs extraSpecialArgs;

      modules = homeModules;
    };
in {
  mkSetup = {
    nixosModules,
    homeModules,
    specialArgs,
    sources,
    host,
    user,
    nix,
    pkgs,
  }: let
    configArgs = specialArgs // {inherit host user nix;};
  in rec
  {
    nixosConfigurations."${host.name}" = mkNixos {
      inherit nixosModules configArgs pkgs;
      inherit (sources) nixpkgs;

      hmConfig = homeConfigurations."${user.alias}".config;
    };

    homeConfigurations."${user.alias}" = mkHome {
      inherit homeModules configArgs pkgs;
      inherit (sources) home-manager;

      nixosConfig = nixosConfigurations."${host.name}".config;
    };
  };
}
