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

    pkgsx.passgen
    hyperfine # benchmarking
    asciinema
    rclone # cloud storages

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
