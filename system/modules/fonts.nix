{
  config,
  pkgs,
  ...
}: {
  fonts = {
    fonts = with pkgs; [
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode" "NerdFontsSymbolsOnly"];})
      cascadia-code
      jetbrains-mono
      roboto-mono
      inconsolata
      roboto
    ];

    fontDir.enable = true;
  };
}