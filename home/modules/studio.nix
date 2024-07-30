{pkgs, ...}: {
  home.packages = with pkgs; [
    (
      figma-linux.overrideAttrs
      (old: {
        nativeBuildInputs = old.nativeBuildInputs ++ [pkgs.copyDesktopItems];
        desktopItems = [
          (
            pkgs.makeDesktopItem {
              name = "Figma (linux)";
              exec = lib.getExe figma-linux;
              icon = "${placeholder "out"}/lib/figma-linux.png";
              desktopName = "Figma (linux)";
              genericName = old.meta.description;
            }
          )
        ];
      })
    )
    shotcut
    gimp
    vlc
  ];

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}
