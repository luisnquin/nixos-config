{
  inputs,
  system,
  ...
}: {
  imports = [
    ./alacritty.nix
  ];

  home.sessionPath = [
    "$HOME/.local/bin"
  ];

  shared.ghostty = {
    enable = true;
    package = inputs.ghostty.packages.${system}.default;
    settings = {
      font-size = "10.8";
    };
  };
}
