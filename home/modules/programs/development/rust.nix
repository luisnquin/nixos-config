{
  pkgs,
  config,
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

  systemd.user.services.rustup-default = {
    Unit = {
      Description = "Set the default Rust toolchain";
    };

    Service = {
      Type = "oneshot";
      ExecStart = "${pkgs.rustup}/bin/rustup default stable";
      Restart = "on-failure";
    };

    Install = {
      WantedBy = ["default.target"];
    };
  };
}
