{
  lib,
  pkgs,
}: let
  colors = {
    normal = "#b5e8e0";
    warning = "#f8bd96";
    critical = "#f28fad";
  };

  rect = x: y: w: h: "M${toString x} ${toString y}h${toString w}v${toString h}h-${toString w}Z";
  px = x: y: w: h: ''<path d="${rect x y w h}"/>'';

  body = px 1 0 17 1 + px 1 12 17 1 + px 0 1 1 11 + px 18 1 1 11 + px 20 4 2 5;
  bolt = lib.concatStrings [(rect 10 3 2 1) (rect 9 4 2 1) (rect 8 5 2 1) (rect 6 6 6 1) (rect 9 7 2 1) (rect 8 8 2 1) (rect 7 9 2 1)];

  # The bolt is cut out of the fill and drawn only where the fill is not, so it
  # reads at any level.
  icon = class: color: charging: fill: let
    level = ''<rect x="2" y="2" width="${toString fill}" height="9"'';
  in
    pkgs.writeText "battery-${class}${lib.optionalString charging "-charging"}-${toString fill}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="22" height="14" viewBox="0 0 22 14" shape-rendering="crispEdges" fill="${color}">
        <defs>
          <mask id="cut"><rect width="22" height="14" fill="#fff"/><path d="${bolt}" fill="#000"/></mask>
          <mask id="gap"><rect width="22" height="14" fill="#fff"/>${level} fill="#000"/></mask>
        </defs>
        ${body}
        ${level}${lib.optionalString charging " mask=\"url(#cut)\""}/>
        ${lib.optionalString charging ''<path d="${bolt}" mask="url(#gap)"/>''}
      </svg>
    '';

  rule = class: color: charging: fill: ''
    #custom-battery.${class}${lib.optionalString charging ".charging"}.fill-${toString fill} {
        background-image: url("${icon class color charging fill}");
    }
  '';
in
  ''
    #custom-battery {
        padding-right: 28px;
        background-repeat: no-repeat;
        background-position: right center;
    }
  ''
  + lib.concatStrings (lib.flatten (lib.mapAttrsToList (class: color:
    map (charging: map (rule class color charging) (lib.range 0 15)) [false true])
  colors))
