{pkgs, ...}: {
  home.packages = with pkgs; [
    osv-scanner
    semgrep
  ];
}
