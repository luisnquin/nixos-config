{
  pkgs,
  libx,
  ...
}: {
  home.packages = with pkgs; [
    kubernetes
    minikube
    kubectl
  ];

  programs = {
    k9s = {
      enable = true;
      skin = libx.formats.fromYAML ../dots/k9s/skin.yml;
      views = libx.formats.fromYAML ../dots/k9s/views.yml;
    };

    zsh.shellAliases = {
      k9s = "k9s --crumbsless --logoless --refresh=2";
      kw = "k9s";
      k = "kubectl";
    };
  };
}
