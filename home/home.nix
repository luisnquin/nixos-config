{
  user,
  nix,
  ...
}: {
  imports = [
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

  # disabledModules = ["misc/news.nix"];

  news.display = "silent";

  xdg = {
    enable = true;
    configFile = {
      "go/env".text = builtins.readFile ./dots/go/env;
    };
  };

  programs.home-manager.enable = true;
}
