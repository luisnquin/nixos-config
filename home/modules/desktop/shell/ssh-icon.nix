{
  lib,
  pkgs,
}: let
  px = x: y: w: h: ''<rect x="${toString x}" y="${toString y}" width="${toString w}" height="${toString h}"/>'';
  diagonal = f: lib.concatMapStrings (i: px i (f i) 1 1) (lib.range 0 5);

  arrows = {
    "in" = {
      color = "#b5e8e0";
      shape = diagonal (i: i) + px 2 6 5 1 + px 6 2 1 4;
    };
    out = {
      color = "#d8b4fe";
      shape = diagonal (i: i + 1) + px 0 0 5 1 + px 0 1 1 4;
    };
    idle = {
      color = "#6c7086";
      shape = arrows."in".shape;
    };
  };

  label = import ./pixel-label.nix {inherit lib;} "SSH";
  labelIcon = pkgs.writeText "ssh-label.svg" ''
    <svg xmlns="http://www.w3.org/2000/svg" width="5" height="${toString (label.height + 1)}" viewBox="0 0 5 ${toString (label.height + 1)}" shape-rendering="crispEdges" fill="#b5e8e0">${label.rects}</svg>
  '';

  icon = class: {color, shape}:
    pkgs.writeText "ssh-${class}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="7" height="8" viewBox="0 0 7 8" shape-rendering="crispEdges" fill="${color}">${shape}</svg>
    '';

  rule = class: arrow: ''
    #custom-ssh-solo.${class},
    #custom-ssh-in.${class},
    #custom-ssh-out.${class} {
        color: ${arrow.color};
        background-image: url("${icon class arrow}");
    }
  '';
in
  ''
    #ssh {
        padding-left: 15px;
        background-image: url("${labelIcon}");
        background-repeat: no-repeat;
        background-position: 6px center;
    }

    #custom-ssh-solo,
    #custom-ssh-in,
    #custom-ssh-out {
        background-repeat: no-repeat;
        background-position: left center;
    }
  ''
  + lib.concatStrings (lib.mapAttrsToList rule arrows)
