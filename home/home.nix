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

  xdg = {
    enable = true;
    configFile = {
      "go/env".text = builtins.readFile ./dots/go/env;
    };
  };

  programs.home-manager.enable = true;
}
