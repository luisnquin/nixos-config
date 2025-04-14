{pkgs, ...}: {
  home.packages = with pkgs; [
    osv-scanner
    aircrack-ng
    # error: cargoHash, cargoVendorDir, cargoDeps, or cargoLock must be set
    # pkgs.extra.havn
    semgrep
    nmap
  ];
}
