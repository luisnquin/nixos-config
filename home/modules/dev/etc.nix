{pkgs, ...}: {
  home.packages = with pkgs; [
    nodePackages.firebase-tools
    nodePackages_latest.cspell
    license-generator
    # onlyoffice-bin
    gnumeric

    hyperfine # benchmarking
    asciinema
    argocd
    rclone # cloud storages
    awscli
    ngrok

    # Command runners
    gnumake
    just

    kondo # Clean up

    minify # HTML, CSS, and JavaScript minifier
    csvkit
    shfmt
    scc
  ];
}
