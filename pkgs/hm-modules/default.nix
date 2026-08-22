# Every Home Manager module the vendored packages define, behind one import.
# Discovered from ./pkgs/<project>/hm-modules, so a new package wires itself in
# without touching the root flake.
let
  root = ../.;
  entries = builtins.readDir root;

  projects =
    builtins.filter
    (
      name:
        entries.${name}
        == "directory"
        && builtins.pathExists (root + "/${name}/hm-modules")
    )
    (builtins.attrNames entries);
in {
  imports = map (name: root + "/${name}/hm-modules") projects;
}
