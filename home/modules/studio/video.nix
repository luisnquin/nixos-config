{pkgs, ...}: {
  home.packages = [
    pkgs.davinci-resolve
    pkgs.shotcut
    pkgs.ffmpeg
  ];

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}
