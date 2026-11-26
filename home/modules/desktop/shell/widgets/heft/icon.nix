{
  lib,
  pkgs,
}: let
  colors = {
    ample = "#b5e8e0";
    tight = "#f8bd96";
    cramped = "#f28fad";
    loading = "#b5e8e0";
  };
  cols = 2;
  rows = 8;
  cells = cols * rows;
  label = import ../../pixel-label.nix {inherit lib;} "DISK";
  left = 7;
  width = left + 7;
  height = 32;
  top = 9;

  px = x: y: w: h: ''<rect x="${toString x}" y="${toString y}" width="${toString w}" height="${toString h}"/>'';

  drum = px (left + 1) 0 5 1 + px left 1 7 1 + px left 2 1 3 + px (left + 6) 2 1 3 + px (left + 1) 5 5 1 + px (left + 4) 3 1 1;

  # Fills bottom-up, a row at a time, left to right.
  cell = level: k: let
    x = left + lib.mod k cols * 4;
    y = top + (rows - 1 - k / cols) * 3;
  in
    ''<rect x="${toString x}" y="${toString y}" width="3" height="2"${lib.optionalString (k >= level) " fill-opacity=\"0.25\""}/>'';

  icon = class: color: level:
    pkgs.writeText "heft-${class}-${toString level}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="${toString width}" height="${toString height}" viewBox="0 0 ${toString width} ${toString height}" shape-rendering="crispEdges" fill="${color}">
        <g transform="translate(0 ${toString ((height - label.height) / 2)})">${label.rects}</g>
        ${drum}
        ${lib.concatMapStrings (cell level) (lib.range 0 (cells - 1))}
      </svg>
    '';

  rule = class: color: level: ''
    #custom-heft.${class}.level-${toString level} {
        background-image: url("${icon class color level}");
    }
  '';
in
  ''
    #custom-heft {
        min-width: ${toString width}px;
        background-repeat: no-repeat;
        background-position: center;
    }
  ''
  + lib.concatStrings (lib.flatten (lib.mapAttrsToList (class: color: map (rule class color) (lib.range 0 cells)) colors))
