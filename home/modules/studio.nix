{pkgs, ...}: {
  home.packages = with pkgs; [
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
