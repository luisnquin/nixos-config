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
  left = 10;
  width = left + cols * 4 - 1;
  height = rows * 4 - 1;

  # Fills bottom-up, a row at a time, left to right.
  cell = color: level: k: let
    x = left + lib.mod k cols * 4;
    y = (rows - 1 - k / cols) * 4;
  in
    if k < level
    then ''<rect x="${toString x}" y="${toString y}" width="3" height="3" fill="${color}"/>''
    else ''<rect x="${toString x}.5" y="${toString y}.5" width="2" height="2" fill="none" stroke="${color}" stroke-opacity="0.4"/>'';

  icon = class: color: level:
    pkgs.writeText "heft-${class}-${toString level}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="${toString width}" height="${toString height}" viewBox="0 0 ${toString width} ${toString height}">
        <text transform="translate(7 ${toString (height / 2)}.5) rotate(-90)" text-anchor="middle" font-family="IoskeleyMono Nerd Font Mono, monospace" font-size="8" font-weight="bold" letter-spacing="0.5" fill="${color}">DISK</text>
        ${lib.concatMapStrings (cell color level) (lib.range 0 (cells - 1))}
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
