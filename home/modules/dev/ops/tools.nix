{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    pkgsx.encore
    cloudflared
    argocd
    lego
  ];
}
