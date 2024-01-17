{pkgs, ...}: let
  inherit (pkgs) nano;
in {
  home = {
    packages = [nano];

    file.".nanorc".text = ''
      include "${nano}/share/nano/*.nanorc"
      include "${nano}/share/nano/extra/*.nanorc"

      set titlecolor white,magenta
      set positionlog
      set autoindent
      set tabsize 4
      set atblanks
      set zero
    '';
  };
}
