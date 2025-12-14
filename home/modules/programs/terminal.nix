{
  inputs,
  system,
  ...
}: {
  shared = {
    alacritty.enable = true;
    ghostty = {
      enable = true;
      package = inputs.ghostty.packages.${system}.default;
      settings = {
        font-size = "10.8";
      };
    };
  };

  home = {
    sessionPath = [
      "$HOME/.local/bin"
    ];
  };
}
