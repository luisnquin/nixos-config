# Everything this repo defines itself, reachable as `pkgs.<name>` through the
# overlay in overlays/nixpkgs.nix. First-party packages keep their source
# vendored in-tree; ./third-party holds derivations over fetched upstreams.
pkgs:
{
  cliphizt = pkgs.callPackage ./clipz/cliphizt {};
  cliplenz = pkgs.callPackage ./clipz/cliplenz {};
  heft = pkgs.callPackage ./heft {};
  herdr-autoname = pkgs.callPackage ./herdr-autoname {};
  phone = pkgs.callPackage ./phone {};
  setup = pkgs.callPackage ./setup {};
  voice-gateway = pkgs.callPackage ./voice-gateway {};
}
// import ./third-party pkgs
