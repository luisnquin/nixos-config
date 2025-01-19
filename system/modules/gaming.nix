{
  pkgs,
  user,
  ...
}: {
  programs = {
    steam = {
      enable = true;
      gamescopeSession.enable = true;
    };

    gamemode.enable = true;
  };

  environment = {
    systemPackages = [
      pkgs.protonup # ensure to run it the first time
    ];

    sessionVariables = {
      STEAM_EXTRA_COMPAT_TOOLS_PATHS = "/home/${user.alias}/.steam/root/compatibilitytools.d";
    };
  };
}