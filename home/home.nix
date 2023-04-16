{
  spicetify-nix,
  pkgs,
  lib,
  ...
}: let
  owner = import ../owner.nix;
  spicePkgs = spicetify-nix.packages.${pkgs.system}.default;
in {
  home = with owner; {
    stateVersion = "23.05";
    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/${username}";
    username = "${username}";
  };

  xdg = {
    enable = true;
    configFile = {
      "go/env".text = builtins.readFile ./dots/go/env;
    };
  };

  programs.home-manager.enable = true;
}
