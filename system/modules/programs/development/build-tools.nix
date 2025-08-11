{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    cmake
    bison
    flex
    openssl
  ];
}
