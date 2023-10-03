{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    rust-analyzer
    rustfmt
    rustup
    clippy
    cargo
    rustc
  ];
}
