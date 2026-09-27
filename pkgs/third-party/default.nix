# Upstream programs packaged here instead of pulled from a shared package flake:
# each one is a derivation over a fetched source, so a bump is a version and a
# hash in a single file. First-party packages sit one level up.
pkgs: {
  davinci-resolve-mcp = pkgs.callPackage ./davinci-resolve-mcp {};
  herdr-pluck = pkgs.callPackage ./herdr-pluck {};
  herdr-sesh = pkgs.callPackage ./herdr-sesh {};
  knots = pkgs.callPackage ./knots {};
  linear-tui = pkgs.callPackage ./linear-tui {};
  px0 = pkgs.callPackage ./px0 {};
  vimix-gtk-themes = pkgs.callPackage ./vimix-gtk-themes {};
}
