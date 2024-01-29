{
  config,
  pkgs,
  lib,
  ...
}:
with lib; let
  cfg = config.programs.nest-cli;
in {
  options = {
    programs.nest-cli = {
      enable = mkEnableOption "nest-cli";
      enableZshIntegration = mkOption {
        default = false;
        type = types.bool;
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [pkgs.nest-cli];

    programs.zsh.plugins = mkIf cfg.enableZshIntegration [
      {
        name = "nestjs-cli-completion";
        file = "_nest";
        src = pkgs.fetchFromGitHub rec {
          name = "${owner}-${repo}-source";
          owner = "filipekiss";
          repo = "nestjs-cli-completion";
          rev = "e66d63b1814b011a57b0d44e03544ddb75267fd2";
          hash = "sha256-nRIQvvEMuGg7flwOsrgYH1U2zomkejn0lGOqmkNhPyM=";
        };
      }
    ];
  };
}
