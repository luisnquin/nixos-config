{
  config,
  lib,
  ...
}: {
  home.sessionVariablesExtra = ''
    export PATH="$PATH:${lib.concatMapStringsSep ":" (dir: "${config.home.homeDirectory}/${dir}") [
      ".cargo/bin"
      ".npm-global/bin"
      ".bun-global/bin"
      "go/bin"
      ".local/bin"
      ".local/share/flatpak/exports/bin"
    ]}"
  '';
}
