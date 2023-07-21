{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    nodePackages.typescript
    nodePackages.pnpm
    nodejs-18_x
    deno
    bun
  ];
}
