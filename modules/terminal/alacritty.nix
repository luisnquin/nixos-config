{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  home-manager.users."${owner.username}" = {
    programs.alacritty = {
      enable = true;
      package = pkgs.alacritty;
    };

    xdg.configFile = {
      "alacritty.yml".text = builtins.readFile ../../dots/home/alacritty.yml;
    };
  };
}
