{
  inputs,
  system,
  ...
}: {
  home.sessionPath = [
    "$HOME/.local/bin"
  ];

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
}
