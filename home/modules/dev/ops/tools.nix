{
  inputs,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    inputs.encore.packages.${pkgs.system}.encore
    argocd
    lego
  ];
}
