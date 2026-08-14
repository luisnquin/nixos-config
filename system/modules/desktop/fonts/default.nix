{pkgs, ...}: let
  ioskeleyMonoCustom =
    pkgs.runCommand "ioskeley-mono-custom" {
      nativeBuildInputs = [pkgs.fontforge];
    } ''
      mkdir -p $out/share/fonts/truetype

      for font in ${pkgs.ioskeley-mono.normal-NF}/share/fonts/truetype/*.ttf; do
        output="$out/share/fonts/truetype/$(basename "$font")"
        fontforge -lang=py -script ${./patch-custom-glyphs.py} \
          "$font" ${./tailscale.svg} ${./memory.svg} \
          ${./ssh-outbound.svg} ${./ssh-inbound.svg} "$output"
      done
    '';
in {
  fonts = {
    packages = with pkgs; [
      nerd-fonts.fira-code
      nerd-fonts.symbols-only
      coders-crux
      ioskeleyMonoCustom
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
