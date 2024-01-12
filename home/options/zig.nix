{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.zig;
in {
  options = {
    programs.zig = {
      enable = mkEnableOption "zig";
      package = mkPackageOption pkgs "zig" {};
      enableZshIntegration = mkOption {
        default = false;
        type = types.bool;
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    programs.zsh.plugins = mkIf cfg.enableZshIntegration [
      {
        name = "zig-zsh-completions-plugin";
        file = "zig-shell-completions.plugin.zsh";
        src = pkgs.fetchFromGitHub {
          owner = "ziglang";
          repo = "shell-completions";
          rev = "31d3ad12890371bf467ef7143f5c2f31cfa7b7c1";
          sha256 = "1fzl1x56b4m11wajk1az4p24312z7wlj2cqa3b519v30yz9clgr0";
        };
      }
    ];
  };
}
