{
  programs.zed-editor = {
    enable = true;
    extensions = ["nix" "toml" "rust"];

    userSettings = {
      telemetry = {
        diagnostics = false;
        metrics = false;
      };

      auto_update = false;
    };
  };
}
