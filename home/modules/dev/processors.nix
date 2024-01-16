{pkgs, ...}: {
  home.packages = with pkgs; [
    htmlq
    yq-go
    jq
  ];
}
