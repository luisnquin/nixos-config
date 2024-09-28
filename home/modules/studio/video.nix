{pkgs, ...}: {
  home.packages = [
    pkgs.shotcut
  ];

  programs.obs-studio = {
    enable = true;

    plugins = with pkgs; [
      obs-studio-plugins.wlrobs
    ];
  };
}
