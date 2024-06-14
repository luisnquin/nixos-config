{pkgs, ...}: {
  home.packages = with pkgs; [
    litecli
    sqlc
  ];
}
