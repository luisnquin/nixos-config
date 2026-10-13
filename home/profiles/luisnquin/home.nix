{
  pkgs,
  user,
  nix,
  ...
}: {
  imports = [
    ../../modules
  ];

  home = {
    inherit (nix) stateVersion;

    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/${user.alias}";
    username = "${user.alias}";

    packages = [pkgs.home-manager];

    file.".face".source = ./.face;

    sessionVariables = {
      DO_NOT_TRACK = 1;
      ENABLE_TELEMETRY = 0;
      TELEMETRY_ENABLED = 0;
      CHECKPOINT_DISABLE = 1;
      DISABLE_EAS_ANALYTICS = 1;
      EXPO_NO_TELEMETRY = 1;
      GH_TELEMETRY = 0;
    };
  };

  news.display = "silent";

  programs.home-manager.enable = false;
}
