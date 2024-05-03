{pkgs, ...}: {
  environment.systemPackages = [pkgs.systemctl-tui];
}
