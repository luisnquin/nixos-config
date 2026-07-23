{
  pkgs,
  lib,
  ...
}: {
  home.packages = with pkgs; [terraform terraformer];

  # After nixpkgs-extra's completionInit: bashcompinit needs its compinit.
  programs.zsh.completionInit = lib.mkAfter ''
    autoload -Uz bashcompinit && bashcompinit
    complete -C ${lib.getExe pkgs.terraform} terraform
  '';
}
