{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    nodePackages.firebase-tools
    nodePackages_latest.cspell
    license-generator
    onlyoffice-bin

    hyperfine # Benchmarking tool
    asciinema
    redoc-cli
    postman
    gnumake
    argocd
    rclone # Cloud storages in one CLI
    awscli
    pgweb
    ngrok
    clang
    just

    # haskellPackages.NanoID
    zathura
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
