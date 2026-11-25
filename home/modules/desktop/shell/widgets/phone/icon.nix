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
  left = 10;
  width = left + 7;
  height = 31;

  square = n: i: state: let
    y = (height - (n * 10 - 3)) / 2 + i * 10;
  in
    if state == "idle" || state == "granted"
    then ''<rect x="${toString left}.5" y="${toString y}.5" width="6" height="6" fill="none" stroke="${teal}"${lib.optionalString (state == "idle") " stroke-opacity=\"0.4\""}/>''
    else ''<rect x="${toString left}" y="${toString y}" width="7" height="7" fill="${moods.${state}}"/>'';

  icon = mood: seq:
    pkgs.writeText "phone-${mood}-${lib.concatStringsSep "-" seq}.svg" ''
      <svg xmlns="http://www.w3.org/2000/svg" width="${toString width}" height="${toString height}" viewBox="0 0 ${toString width} ${toString height}">
        <text transform="translate(7 ${toString (height / 2)}.5) rotate(-90)" text-anchor="middle" font-family="IoskeleyMono Nerd Font Mono, monospace" font-size="8" font-weight="bold" letter-spacing="0.5" fill="${moods.${mood}}"${lib.optionalString (seq == []) " fill-opacity=\"0.4\""}>PHONE</text>
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
