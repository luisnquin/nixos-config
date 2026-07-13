{pkgs, ...}: let
  ioskeleyMonoTailscale = pkgs.runCommand "ioskeley-mono-tailscale" {
    nativeBuildInputs = [pkgs.fontforge];
  } ''
    mkdir -p $out/share/fonts/truetype

    for font in ${pkgs.ioskeley-mono.normal-NF}/share/fonts/truetype/*.ttf; do
      output="$out/share/fonts/truetype/$(basename "$font")"
      fontforge -lang=py -script ${./patch-tailscale.py} \
        "$font" ${./tailscale.svg} "$output"
    done
  '';
in {
  fonts = {
    packages = with pkgs; [
      nerd-fonts.fira-code
      nerd-fonts.symbols-only
      coders-crux
      ioskeleyMonoTailscale
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
