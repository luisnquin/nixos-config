{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    nodePackages.firebase-tools
    nodePackages_latest.cspell
    license-generator
    onlyoffice-bin

    pkgsx.pg-ping
    hyperfine # benchmarking
    asciinema
    redoc-cli
    postman
    gnumake
    argocd
    rclone # cloud storages
    awscli
    pgweb
    ngrok
    just

    minify # HTML, CSS, and JavaScript minifier
    shfmt
    sqlc
    scc

    csvkit
    htmlq
    yq-go
    jq
  ];
}
