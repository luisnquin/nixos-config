{pkgs, ...}: {
  home.packages = with pkgs; [
    sqlc
  ];
}
