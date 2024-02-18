{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    npm-check-updates # (ncu) Find newer versions of package dependencies and check outdated npm packages locally or globally.
    pkgsx.npkill # remove node_modules from child directories
  ];
}
