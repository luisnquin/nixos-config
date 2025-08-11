{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    stdenv_32bit
    libsecret
  ];
}
