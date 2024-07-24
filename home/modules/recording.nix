{pkgs, ...}: {
  home.packages = with pkgs; [
    shotcut
    vlc
  ];

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}
