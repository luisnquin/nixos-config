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

  # cursor is trying to autoupdate _._
}
