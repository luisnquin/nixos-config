{
  pkgs,
  libx,
  user,
  ...
}: {
  environment = {
    systemPackages = [
      pkgs.protonup-ng # ensure to run it the first time
      pkgs.steam-run
    ];

    sessionVariables = {
      STEAM_EXTRA_COMPAT_TOOLS_PATHS = "/home/${user.alias}/.steam/root/compatibilitytools.d";
    };
  };

  programs = {
    steam = {
      enable = true;
      package = pkgs.steam.override {extraEnv = libx.nvidia.offloadEnv;};
      gamescopeSession.enable = true;
    };

    gamemode.enable = true;
  };
}
