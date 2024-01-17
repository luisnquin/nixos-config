{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    nodePackages.pnpm
    pkgsx.npkill # remove node_modules from child directories
    ncu #  Find newer versions of package dependencies and check outdated npm packages locally or globally.
    nodejs_21
    bun
  ];
}
