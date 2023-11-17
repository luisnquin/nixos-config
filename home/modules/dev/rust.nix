{pkgs, ...}: {
  home.file.".cargo/config.toml".source = (pkgs.formats.toml {}).generate "cargo-config" {
    net = {
      git-fetch-with-cli = true;
    };
  };
}
