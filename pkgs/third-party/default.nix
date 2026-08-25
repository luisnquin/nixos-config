# Upstream programs packaged here instead of pulled from a shared package flake:
# each one is a derivation over a fetched source, so a bump is a version and a
# hash in a single file. First-party packages sit one level up.
pkgs: {
  freebuff = pkgs.callPackage ./freebuff {};
  herdr-pluck = pkgs.callPackage ./herdr-pluck {};
  herdr-sesh = pkgs.callPackage ./herdr-sesh {};
  linear-tui = pkgs.callPackage ./linear-tui {};
  spiceedit = pkgs.callPackage ./spiceedit {};
  vimix-gtk-themes = pkgs.callPackage ./vimix-gtk-themes {};
}
