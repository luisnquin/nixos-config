{pkgs, ...}: {
  home.packages = with pkgs; [
    osv-scanner
    aircrack-ng
    semgrep
  ];
}
