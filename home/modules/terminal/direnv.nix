{
  programs.direnv = {
    enable = true;
    enableZshIntegration = true;
    enableBashIntegration = true;
    config = {
      global = {
        load_dotenv = true;
        warn_timeout = "5s";
      };

      whitelist = {
        prefix = [
          "$HOME/Projects/github.com/Qompa-Fi"
          "$HOME/Projects/github.com/luisnquin"
          "$HOME/Projects/github.com/chanchitaapp"
          "$HOME/Projects/github.com/0xc000022070"
          "$HOME/.dotfiles"
        ];
      };
    };

    nix-direnv.enable = true;
  };

  home.sessionVariables = {
    DIRENV_LOG_FORMAT = "direnv: %s";
  };
}
