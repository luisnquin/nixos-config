{
  pkgsx,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    nodePackages.typescript
    nodePackages.pnpm
    pkgsx.npkill
    nodejs-18_x
    deno
    bun
  ];
}
