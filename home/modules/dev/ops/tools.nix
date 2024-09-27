{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.encore
    argocd
    lego
  ];
}
