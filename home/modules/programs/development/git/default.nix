{
  config,
  lib,
  pkgs,
  user,
  ...
}: let
  inherit (config.programs.git) hooks;

  preCommit = pkgs.writeShellScript "git-pre-commit" ''
    set -e

    ${lib.getExe pkgs.knots} --staged -l rust --cognitive-threshold 15 --mccabe-threshold 15 --find-duplicates
    exec ${hooks.pre-commit} "$@"
  '';
in {
  imports = [
    ./terminal.nix
    ./jujutsu.nix
    ./github.nix
    ./etc.nix
  ];

  shared = {
    git = {
      enable = true;
      user = {
        name = user.fullName;
        email = user.gitEmail;
      };
    };
    lazygit.enable = true;
  };

  programs.git.iniContent.core.hooksPath = lib.mkForce (toString (
    pkgs.linkFarm "git-hooks" (
      lib.mapAttrsToList (name: path: {inherit name path;}) (hooks // {pre-commit = preCommit;})
    )
  ));
}
