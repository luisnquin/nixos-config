{ ...}: {
  programs.direnv = {
    enable = true;
    enableZshIntegration = true;
    enableBashIntegration = true;
    config = {
      global = {
        load_dotenv = true;
        warn_timeout = "5s";
      };
    };

    nix-direnv.enable = true;
  };
}
