{
  pkgs,
  lib,
  ...
}: let
  inherit (pkgs) terraform;
in {
  home.packages = [terraform];

  programs.zsh.completionInit = ''
    complete -C ${lib.getExe terraform} terraform
  '';
}
