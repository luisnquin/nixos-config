{
  user,
  nix,
  ...
}: {
  require = [
    ./services
    ./modules
  ];

  home = {
    inherit (nix) stateVersion;

    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/${user.alias}";
    username = "${user.alias}";

    file.".face".source = ./dots/.face;
  };

  # I'm not using standard paths
  news.display = "silent";

  xdg = {
    enable = true;
    configFile = {
      "go/env".text = builtins.readFile ./dots/go/env;
    };
  };

  programs.home-manager.enable = true;
}
