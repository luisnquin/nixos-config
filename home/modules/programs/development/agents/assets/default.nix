{lib}: let
  root = builtins.path {
    name = "claude-agent-assets";
    path = ./.;
  };

  stripExt = name: let
    m = builtins.match "(.+)\\.[^.]+$" name;
  in
    if m == null
    then name
    else builtins.head m;

  read = subdir:
    lib.mapAttrs'
    (name: _: lib.nameValuePair (stripExt name) "${root}/${subdir}/${name}")
    (builtins.readDir (./. + "/${subdir}"));
in {
  sounds = read "sounds";
  images = read "images";
}
