# Everything this repo defines itself, reachable as `pkgs.<name>` through the
# overlay in overlays/nixpkgs.nix. First-party packages keep their source
# vendored in-tree; ./third-party holds derivations over fetched upstreams.
pkgs:
{
  ee-workbench = pkgs.callPackage ./ee-workbench {};
  hark = pkgs.callPackage ./hark {};
  heft = pkgs.callPackage ./heft {};
  herdr-autoname = pkgs.callPackage ./herdr-autoname {};
  herdr-recall = pkgs.callPackage ./herdr-recall {};
  mcp-gateway = pkgs.callPackage ./mcp-gateway {};
  phone = pkgs.callPackage ./phone {};
  pinentry-gate = pkgs.callPackage ./pinentry-gate {};
  setup = pkgs.callPackage ./setup {};
  barfeed = pkgs.callPackage ./barfeed {};
  ttygate = pkgs.callPackage ./ttygate {};
}
// import ./third-party pkgs
