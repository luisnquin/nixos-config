{
  pkgs-extra,
  pkgs,
  ...
}: {
  fonts = {
    packages = with pkgs; [
      nerd-fonts.fira-code
      nerd-fonts.symbols-only
      pkgs-extra.coders-crux
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
