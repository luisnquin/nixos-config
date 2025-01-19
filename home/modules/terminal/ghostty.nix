{
  inputs,
  system,
  ...
}: {
  programs.ghostty = {
    enable = true;
    package = inputs.ghostty.packages.${system}.default;
    enableZshIntegration = true;
    enableBashIntegration = true;

    settings = {
      font-synthetic-style = "bold";
      font-size = "10.8";

      theme = "iceberg-dark";

      cursor-style = "bar";
      cursor-style-blink = "true";

      background-opacity = "0.8";
      background-blur-radius = "20";

      link-url = "true";
      class = "com.mitchellh.ghostty";

      working-directory = "home";

      window-padding-x = "15,4";
      window-padding-y = "8,6";

      # https://ghostty.org/docs/config/reference#window-vsync
      window-vsync = "false";
      window-decoration = "false";

      window-save-state = "never";

      clipboard-read = "allow";
      clipboard-write = "allow";

      clipboard-trim-trailing-spaces = "true";
      clipboard-paste-protection = "true";

      shell-integration = "zsh";

      desktop-notifications = "true";

      auto-update = "off";
      auto-update-channel = "tip";

      # Saves screen in tpm file and opens it with the default app (in this case, Zen Browser)
      keybind = "ctrl+f=write_screen_file:open";
    };
  };
}
