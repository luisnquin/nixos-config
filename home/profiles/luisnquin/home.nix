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
      ENABLE_TELEMETRY = 0;
      TELEMETRY_ENABLED = 0;
    };
  };

  news.display = "silent";

  programs.home-manager.enable = false;
}
