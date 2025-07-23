{
  pkgs,
  lib,
  ...
}: {
  home.packages = with pkgs; [terraform terraformer];

  programs.zsh.completionInit = ''
    complete -C ${lib.getExe pkgs.terraform} terraform
  '';
}
