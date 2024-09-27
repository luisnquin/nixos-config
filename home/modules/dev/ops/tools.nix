{pkgs, ...}: {
  home.packages = with pkgs; [
    pkgs.extra.encore
    argocd
    lego
  ];
}
