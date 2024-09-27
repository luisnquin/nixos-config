{
  config,
  pkgs,
  lib,
  ...
}: let
  cfg = config.programs.emoji-fzf;
in
  with lib; {
    options = {
      programs.emoji-fzf.enable = mkEnableOption "emoji-fzf";
    };

    config = mkIf cfg.enable {
      home.packages = [
        pkgs.extra.emoji-fzf
      ];

      programs.zsh.plugins = [
        {
          name = "emoji-fzf";
          file = "emoji-fzf.zsh";
          src = pkgs.fetchFromGitHub {
            owner = "pschmitt";
            repo = "emoji-fzf.zsh";
            rev = "958e4f60a8c43823f35f0bd29b06a08cd0bdea0e";
            hash = "sha256-fF9Sm1q7k9bWLeMt3Uhb5WbH3LU7RpnFCZMgaYSv77g=";
          };
        }
      ];
    };
  }
