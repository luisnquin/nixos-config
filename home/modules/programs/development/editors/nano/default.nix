{pkgs, ...}: {
  home = {
    packages = [pkgs.nano];

    file.".nanorc".text = ''
      include "${pkgs.nano}/share/nano/*.nanorc"
      include "${pkgs.nano}/share/nano/extra/*.nanorc"
      include "${./.nanorc}"

      set titlecolor white,magenta
      set positionlog
      set autoindent
      set tabsize 4
      set atblanks
      set zero
    '';
  };
}
