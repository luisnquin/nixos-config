{pkgs, ...}: {
  decode = encodedStr: let
    outFile = pkgs.runCommand "decode-base64" {} "echo '${encodedStr}' | base64 --decode > $out";
  in
    builtins.readFile outFile;
}
