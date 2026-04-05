{
  config,
  pkgs,
  lib,
  ...
}: {
  home = {
    packages = with pkgs; let
      cargo-plugins = [
        cargo-sweep
      ];
    in
      [
        rust-analyzer
        rustfmt
        clippy

        rustc
      ]
      ++ cargo-plugins;

    sessionPath = ["$HOME/.cargo/bin"];
  };

  programs.cargo = {
    enable = true;
    cargoHome = "${config.xdg.dataHome}/.cargo";

    settings = {
      net = {
        git-fetch-with-cli = true;
      };

      build = {
        jobs = 4;
        incremental = true;
        target-dir = "target";
      };

      profile = {
        dev = {
          incremental = true;
          codegen-units = 256;
          debug = 1;
        };

        release = {
          incremental = true;
          codegen-units = 64;
          lto = false;
          debug = 0;
          strip = "debuginfo";
        };
      };

      future-incompat-report = {
        frequency = "always";
      };

      cache = {
        auto-clean-frequency = "1 day";
      };

      term = {
        quiet = false;
        verbose = false;
        color = "auto";
        hyperlinks = true;
        unicode = true;
      };
    };
  };

  home.activation.rustupDefault = lib.hm.dag.entryAfter ["writeBoundary"] ''
    ${lib.getExe pkgs.rustup} default stable >/dev/null 2>&1 || true
  '';
}
