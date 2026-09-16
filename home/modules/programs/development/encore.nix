{
  home.sessionVariables.DISABLE_ENCORE_TELEMETRY = 1;

  programs.encore = {
    enable = true;
    settings = {
      browser = "never";
    };
  };
}
