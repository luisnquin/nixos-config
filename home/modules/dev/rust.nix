{pkgs, ...}: {
  home.packages = with pkgs; [
    # rust-analyzer
    # rustfmt
    # rustup
    # clippy
    # cargo
    rustc
  ];
}
