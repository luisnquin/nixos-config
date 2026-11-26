# A word in a 3x5 pixel font turned a quarter counter-clockwise, read bottom
# to top: 5px wide, 4px per letter minus the trailing gap.
{lib}: word: let
  glyphs = {
    D = ["110" "101" "101" "101" "110"];
    E = ["111" "100" "110" "100" "111"];
    H = ["101" "101" "111" "101" "101"];
    I = ["111" "010" "010" "010" "111"];
    K = ["101" "101" "110" "101" "101"];
    N = ["101" "111" "111" "101" "101"];
    O = ["111" "101" "101" "101" "111"];
    P = ["110" "101" "110" "100" "100"];
    S = ["111" "100" "111" "001" "111"];
  };
  letters = lib.stringToCharacters word;
  height = lib.length letters * 4 - 1;
  letter = i: ch:
    lib.concatStrings (lib.imap0 (r: row:
      lib.concatStrings (lib.imap0 (c: bit:
        lib.optionalString (bit == "1")
        ''<rect x="${toString r}" y="${toString (height - 1 - (i * 4 + c))}" width="1" height="1"/>'')
      (lib.stringToCharacters row)))
    glyphs.${ch});
in {
  inherit height;
  width = 5;
  rects = lib.concatStrings (lib.imap0 letter letters);
}
