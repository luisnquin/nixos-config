{pkgs, ...}: {
  environment.systemPackages = [
    pkgs.cloudflared
  ];
}
