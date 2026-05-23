{
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

    file.".face".source = ./.face;

    sessionVariables = {
      ENABLE_TELEMETRY = 0;
      TELEMETRY_ENABLED = 0;
    };
  };

  # disabledModules = ["misc/news.nix"];

  news.display = "silent";

  programs.home-manager.enable = true;
}
