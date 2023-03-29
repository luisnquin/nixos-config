{
  spicetify-nix,
  pkgs,
  lib,
  ...
}: let
  owner = import ../owner.nix;
  spicePkgs = spicetify-nix.packages.${pkgs.system}.default;
in {
  home = {
    stateVersion = "23.05";
    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/luisnquin";
    username = "luisnquin";
  };

  xdg = {
    enable = true;
    configFile = with owner; {
      "go/env".text = builtins.readFile ./dots/go/env;

      "openaiapirc".text = ''
        [openai]
        organization_id = ${openai.organization-id}
        secret_key = ${openai.secret-key}
      '';
    };
  };

  programs.home-manager.enable = true;
}
