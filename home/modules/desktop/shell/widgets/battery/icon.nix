{
  lib,
  pkgs,
}: let
  colors = {
    normal = "#b5e8e0";
    warning = "#f8bd96";
    critical = "#f28fad";
  };
  bolt = "M11.5 2.5 7.5 7H10.25L9.5 10.5 13.5 6H10.75Z";

  # The bolt is cut out of the fill and drawn only where the fill is not, so it
  # reads at any level.
  icon = class: color: charging: fill: let
    level = ''<rect x="3" y="3" width="${toString fill}" height="7" rx="1.5"'';
  in
    pkgs.writeText "battery-${class}${lib.optionalString charging "-charging"}-${toString fill}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="24" height="13" viewBox="0 0 24 13">
        <defs>
          <mask id="cut"><rect width="24" height="13" fill="#fff"/><path d="${bolt}"/></mask>
          <mask id="gap"><rect width="24" height="13" fill="#fff"/>${level}/></mask>
        </defs>
        <rect x="0.75" y="0.75" width="19.5" height="11.5" rx="3" fill="none" stroke="${color}" stroke-width="1.5"/>
        <rect x="21" y="4" width="2" height="5" rx="1" fill="${color}"/>
        ${level} fill="${color}"${lib.optionalString charging " mask=\"url(#cut)\""}/>
        ${lib.optionalString charging ''<path d="${bolt}" fill="${color}" mask="url(#gap)"/>''}
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
        padding-right: 30px;
        background-repeat: no-repeat;
        background-position: right center;
    }
  ''
  + lib.concatStrings (lib.flatten (lib.mapAttrsToList (class: color:
    map (charging: map (rule class color charging) (lib.range 0 15)) [false true])
  colors))
