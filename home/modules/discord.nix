{pkgs, ...}: {
  home.packages = [
    (let
      # TODO: replace by "discord-canary"
      discord = pkgs.discord.override {
        withVencord = true;
        withOpenASAR = true;
        nss = pkgs.nss_latest;
      };
    in
      discord.overrideAttrs (old: {
        libPath = old.libPath + ":${pkgs.libglvnd}/lib";
        nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.makeWrapper];
        postFixup = ''
          wrapProgram $out/opt/Discord/Discord --add-flags "\''${NIXOS_OZONE_WL:+\''${WAYLAND_DISPLAY:+--ozone-platform=wayland}}"
        '';
      }))
  ];
}
