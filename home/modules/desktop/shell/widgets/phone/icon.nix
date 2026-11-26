{
  lib,
  pkgs,
}: let
  teal = "#b5e8e0";
  amber = "#f8bd96";
  pink = "#f28fad";
  moods = {
    idle = teal;
    granted = teal;
    held = teal;
    asking = amber;
    locked = pink;
  };
  marks = 3;
  label = import ../../pixel-label.nix {inherit lib;} "PHONE";
  left = 7;
  width = left + 11;
  height = 32;

  px = x: y: w: h: ''<rect x="${toString x}" y="${toString y}" width="${toString w}" height="${toString h}"/>'';

  # Chamfered handset with a speaker slot and a home bar.
  handset = px (left + 1) 0 9 1 + px (left + 1) 31 9 1 + px left 1 1 30 + px (left + 10) 1 1 30 + px (left + 4) 2 3 1 + px (left + 4) 29 3 1;

  square = n: i: state: let
    y = 5 + (22 - (n * 7 - 2)) / 2 + i * 7;
  in
    if state == "idle" || state == "granted"
    then ''<rect x="${toString (left + 3)}.5" y="${toString y}.5" width="4" height="4" fill="none" stroke="${teal}"${lib.optionalString (state == "idle") " stroke-opacity=\"0.4\""}/>''
    else ''<rect x="${toString (left + 3)}" y="${toString y}" width="5" height="5" fill="${moods.${state}}"/>'';

  icon = mood: seq:
    pkgs.writeText "phone-${mood}-${lib.concatStringsSep "-" seq}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="${toString width}" height="${toString height}" viewBox="0 0 ${toString width} ${toString height}" shape-rendering="crispEdges">
        <g fill="${moods.${mood}}"${lib.optionalString (seq == []) " fill-opacity=\"0.4\""}><g transform="translate(0 ${toString ((height - label.height) / 2)})">${label.rects}</g>${handset}</g>
        ${lib.concatStrings (lib.imap0 (square (lib.length seq)) seq)}
      </svg>
    '';

  # Every sequence of up to `marks` device states, each under the mood it implies.
  rank = ["idle" "locked" "granted" "held" "asking"];
  seqsOf = n: if n == 0 then [[]] else lib.concatMap (s: map (t: [s] ++ t) (seqsOf (n - 1))) rank;
  seqs = lib.concatMap seqsOf (lib.range 0 marks);
  moodOf = seq: lib.findFirst (r: lib.elem r seq) "idle" (lib.reverseList rank);

  rule = seq: ''
    #custom-phone.marks-${lib.concatStringsSep "-" seq} {
        background-image: url("${icon (moodOf seq) seq}");
    }
  '';
in
  ''
    #custom-phone {
        min-width: ${toString width}px;
        background-repeat: no-repeat;
        background-position: center;
    }
  ''
  + lib.concatMapStrings rule seqs
