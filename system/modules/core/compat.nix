{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    wineWow64Packages.waylandFull
    stdenv_32bit
    libsecret
  ];
}
