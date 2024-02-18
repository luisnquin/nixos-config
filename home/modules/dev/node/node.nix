# It includes Node.js runtimes and package managers
{pkgs, ...}: {
  home.packages = with pkgs; [
    nodePackages.pnpm
    nodejs_21
    bun
  ];
}
