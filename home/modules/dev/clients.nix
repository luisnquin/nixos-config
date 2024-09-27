{pkgs, ...}: {
  home.packages = with pkgs; [
    insomnia # another REST client
    websocat
  ];
}
