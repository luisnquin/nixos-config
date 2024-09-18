{
  passgen,
  senv,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    nodePackages.firebase-tools
    nodePackages_latest.cspell
    license-generator
    # onlyoffice-bin
    gnumeric

    passgen
    hyperfine # benchmarking
    # asciinema
    rclone # cloud storages

    # Command runners
    gnumake
    just

    ngrok

    senv

    kondo # Clean up

    minify # HTML, CSS, and JavaScript minifier
    csvkit
    shfmt
    scc
  ];
}
