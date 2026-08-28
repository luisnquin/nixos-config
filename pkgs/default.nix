# Everything this repo defines itself, reachable as `pkgs.<name>` through the
# overlay in overlays/nixpkgs.nix. First-party packages keep their source
# vendored in-tree; ./third-party holds derivations over fetched upstreams.
pkgs:
{
  cliphizt = pkgs.callPackage ./clipz/cliphizt {};
  cliplenz = pkgs.callPackage ./clipz/cliplenz {};
  ee-workbench = pkgs.callPackage ./ee-workbench {};
  heft = pkgs.callPackage ./heft {};
  herdr-autoname = pkgs.callPackage ./herdr-autoname {};
  herdr-recall = pkgs.callPackage ./herdr-recall {};
  mcp-gateway = pkgs.callPackage ./mcp-gateway {};
  phone = pkgs.callPackage ./phone {};
  setup = pkgs.callPackage ./setup {};
  waytools = pkgs.callPackage ./waytools {};
  ttygate = pkgs.callPackage ./ttygate {};
}
// import ./third-party pkgs
