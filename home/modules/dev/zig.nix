{
  config,
  pkgs,
  ...
}: {
  # for fun, I guess
  programs.zig = {
    enable = true;
    package = pkgs.zig;
    enableZshIntegration = config.programs.zsh.enable;
  };
}
