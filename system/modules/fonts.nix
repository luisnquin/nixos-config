{pkgs, ...}: {
  fonts = {
    packages = with pkgs; [
      (nerdfonts.override {fonts = ["FiraCode" "CascadiaCode" "NerdFontsSymbolsOnly"];})
      pkgs.extra.coders-crux
      jetbrains-mono
      cascadia-code
      roboto-mono
      inconsolata
      fira-code
      roboto
    ];

    fontDir.enable = true;
  };

  environment.systemPackages = [pkgs.fontpreview];
}
