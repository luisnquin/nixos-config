{pkgs, ...}: {
  home.packages = [
    pkgs.davinci-resolve
    pkgs.shotcut
  ];

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}
