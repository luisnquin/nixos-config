{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    osv-scanner
    aircrack-ng
    pkgsx.havn
    semgrep
    nmap
  ];
}
