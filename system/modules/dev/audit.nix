{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    osv-scanner
    semgrep
  ];
}
