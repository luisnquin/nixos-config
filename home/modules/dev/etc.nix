{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    nodePackages.firebase-tools
    nodePackages_latest.cspell
    license-generator
    # onlyoffice-bin
    gnumeric

    pkgsx.pg-ping
    hyperfine # benchmarking
    asciinema
    redoc-cli
    gnumake
    argocd
    rclone # cloud storages
    awscli
    pgweb
    ngrok
    just

    kondo

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
