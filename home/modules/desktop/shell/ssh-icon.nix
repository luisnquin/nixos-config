{
  lib,
  pkgs,
}: let
  px = x: y: w: h: ''<rect x="${toString x}" y="${toString y}" width="${toString w}" height="${toString h}"/>'';
  diagonal = from: to: lib.concatMapStrings (i: px i i 1 1) (lib.range from to);

  arrows = {
    "in" = {
      color = "#b5e8e0";
      shape = diagonal 0 5 + px 3 6 4 1 + px 6 3 1 3;
    };
    out = {
      color = "#d8b4fe";
      shape = diagonal 1 6 + px 0 0 4 1 + px 0 1 1 3;
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

  icon = height: class: {color, shape}:
    pkgs.writeText "ssh-${class}-${toString height}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="7" height="${toString height}" viewBox="0 0 7 ${toString height}" shape-rendering="crispEdges" fill="${color}">${shape}</svg>
    '';

  rule = class: arrow: ''
    #custom-ssh-solo.${class} {
        color: ${arrow.color};
        background-image: url("${icon 8 class arrow}");
    }

    #custom-ssh-in.${class},
    #custom-ssh-out.${class} {
        color: ${arrow.color};
        background-image: url("${icon 7 class arrow}");
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
