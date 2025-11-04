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
        inherit (pkgs.stdenv.hostPlatform) system;
      };
  in
    nixpkgs.lib.nixosSystem {
      system = pkgs.stdenv.hostPlatform.system;
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
        system = pkgs.stdenv.hostPlatform.system;
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
    profilesPath,
    homeModules,
    specialArgs,
    hostsPath,
    flakes,
    host,
    user,
    nix,
    pkgs,
  }: let
    configArgs = specialArgs // {inherit host user nix;};
  in rec
  {
    nixosConfigurations."${host.name}" = mkNixos {
      inherit configArgs pkgs;
      inherit (flakes) nixpkgs;

      nixosModules =
        nixosModules
        ++ [
          (hostsPath + "/${host.name}")
        ];

      hmConfig = homeConfigurations."${user.alias}".config;
    };

    homeConfigurations."${user.alias}" = mkHome {
      inherit configArgs pkgs;
      inherit (flakes) home-manager;

      homeModules =
        homeModules
        ++ [
          (profilesPath + "/${user.alias}")
        ];

      nixosConfig = nixosConfigurations."${host.name}".config;
    };
  };
}
