{
  pkgs,
  lib,
  ...
}: {
  home = {
    file.".cargo/config.toml".source = (pkgs.formats.toml {}).generate "cargo-config" {
      net = {
        git-fetch-with-cli = true;
      };
    };

    activation.rustupDefault = lib.hm.dag.entryAfter ["writeBoundary"] ''
      ${pkgs.rustup}/bin/rustup default stable
    '';
  };
}
