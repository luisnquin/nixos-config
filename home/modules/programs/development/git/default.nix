{
  lib,
  pkgs,
  user,
  ...
}: {
  imports = [
    ./terminal.nix
    ./jujutsu.nix
    ./github.nix
    ./etc.nix
  ];

  home.packages = [pkgs.knots];

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

  programs.git.settings.hook.knots = {
    event = "pre-commit";
    command = "${lib.getExe pkgs.knots} --staged -l rust --cognitive-threshold 15 --mccabe-threshold 15 --find-duplicates";
  };
}
