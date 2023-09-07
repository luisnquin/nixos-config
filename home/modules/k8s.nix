{pkgs, ...}: {
  xdg.configFile = {
    "k9s/views.yml".text = builtins.readFile ../dots/k9s/views.yml;
    "k9s/skin.yml".text = builtins.readFile ../dots/k9s/skin.yml;
  };

  home.packages = with pkgs; [
    kubernetes
    minikube
    kubectl
    # lens
    k9s
  ];

  programs.zsh.shellAliases = {
    k9s = "k9s --crumbsless --logoless --refresh=2";
    kw = "k9s";
    k = "kubectl";
  };
}
