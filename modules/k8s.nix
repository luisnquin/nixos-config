{
  config,
  pkgs,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  environment.systemPackages = with pkgs; [
    kubernetes
    minikube
    kubectl
    # lens
    k9s
  ];

  home-manager.users."${owner.username}" = {
    xdg.configFile = {
      # I think that large files are more maintainable in this way
      # "k9s/config.yml".text = builtins.readFile ../dots/home/k9s/config.yml;
      "k9s/views.yml".text = builtins.readFile ../dots/home/k9s/views.yml;
      "k9s/skin.yml".text = builtins.readFile ../dots/home/k9s/skin.yml;
    };
  };
}
