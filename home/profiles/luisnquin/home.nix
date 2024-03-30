{
  user,
  nix,
  ...
}: {
  imports = [
    ../../services
    ../../modules
  ];

  home = {
    inherit (nix) stateVersion;

    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/${user.alias}";
    username = "${user.alias}";

    file.".face".source = ./.face;
  };

  # disabledModules = ["misc/news.nix"];

  news.display = "silent";

  programs.home-manager.enable = true;
}
