{pkgs, ...}: {
  home.packages = with pkgs; [
    litecli
    pgcli
    mycli
    sqlc
  ];
}
