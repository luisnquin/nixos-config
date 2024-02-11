{pkgs, ...}: {
  home = {
    packages = with pkgs; [
      rust-analyzer
      rustfmt
      # rustup
      clippy
      cargo
      rustc
    ];

    file.".cargo/config.toml".source = (pkgs.formats.toml {}).generate "cargo-config" {
      net = {
        git-fetch-with-cli = true;
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
