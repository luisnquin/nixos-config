{pkgs, ...}: {
  home.packages = with pkgs; [
    osv-scanner
    aircrack-ng
    pkgs.extra.havn
    semgrep
    nmap
  ];
}
