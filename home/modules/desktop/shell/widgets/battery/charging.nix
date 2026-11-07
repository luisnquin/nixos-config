{
  lib,
  pkgs,
}: let
  mint = "#b5e8e0";
  bolt = "M10 1.5 5.5 6.6H8.4L7.4 10.5 12.5 5H9.6L10.6 1.5Z";

  # The fill is masked around the bolt with a gap, so the bolt reads at any level.
  icon = fill:
    pkgs.writeText "battery-charging-${toString fill}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="26" height="12" viewBox="0 0 26 12">
        <defs><mask id="m"><rect width="20" height="12" fill="#fff"/><path d="${bolt}" stroke="#000" stroke-width="2.2" stroke-linejoin="round"/></mask></defs>
        <rect x="0.5" y="0.5" width="17" height="11" rx="2" fill="none" stroke="${mint}"/>
        <rect x="18" y="4" width="1.5" height="4" rx="0.5" fill="${mint}"/>
        <rect x="2" y="2" width="${toString fill}" height="8" rx="0.75" fill="${mint}" mask="url(#m)"/>
        <path d="${bolt}" fill="${mint}"/>
      </svg>
    '';
in
  ''
    #custom-battery.charging {
        padding-right: 30px;
        background-repeat: no-repeat;
        background-position: right center;
    }
  ''
  + lib.concatMapStrings (fill: ''
    #custom-battery.charging.fill-${toString fill} {
        background-image: url("${icon fill}");
    }
  '') (lib.range 0 13)
