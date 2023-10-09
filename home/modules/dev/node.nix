{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    nodePackages.pnpm
    pkgsx.npkill # remove node_modules from child directories
    nodejs-18_x
    bun
  ];
}
