{
  pkgs,
  pkgsx,
  ...
}: {
  fonts = {
    packages = with pkgs; [
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode" "NerdFontsSymbolsOnly"];})
      pkgsx.coders-crux
      jetbrains-mono
      cascadia-code
      roboto-mono
      inconsolata
      roboto
    ];

    fontDir.enable = true;
  };

  environment.systemPackages = [pkgs.fontpreview];
}
