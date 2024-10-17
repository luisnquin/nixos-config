{
  config,
  pkgs,
  ...
}: {
  programs.jujutsu = {
    enable = true;
    package = pkgs.jujutsu;

    # Full options:
    # https://github.com/martinvonz/jj/blob/main/docs/config.md
    settings = {
      user = {
        email = config.programs.git.userEmail;
        name = config.programs.git.userName;
      };
    };
  };
}
