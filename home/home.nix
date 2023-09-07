{
  user,
  nix,
  ...
}: {
  require = [
    ./modules
  ];

  home = {
    inherit (nix) stateVersion;

    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/${user.alias}";
    username = "${user.alias}";
  };

  # I'm not using standard paths
  news.display = "silent";

  home.sessionVariables = {
    "DOTFILES_PATH" = "/home/${user.alias}/.dotfiles";
  };

  xdg = {
    enable = true;
    configFile = {
      "go/env".text = builtins.readFile ./dots/go/env;
    };
  };

  programs.home-manager.enable = true;
}
