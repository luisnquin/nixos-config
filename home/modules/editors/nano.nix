{pkgs, ...}: let
  inherit (pkgs) nano;
in {
  home = {
    packages = [nano];

    file.".nanorc".text = let
      nanoDir = builtins.path {
        name = "personal-nanorc-files";
        path = ./dots/nanorc;
      };
    in ''
      include "${nano}/share/nano/*.nanorc"
      include "${nano}/share/nano/extra/*.nanorc"
      include "${nanoDir}/*.nanorc"

      set titlecolor white,magenta
      set positionlog
      set autoindent
      set tabsize 4
      set atblanks
      set zero
    '';
  };
}
