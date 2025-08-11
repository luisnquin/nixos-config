{pkgs, ...}: {
  programs.thunar = {
    enable = true;
    plugins = with pkgs.xfce; [
      thunar-volman
    ];
  };

  services = {
    gvfs.enable = true;
    tumbler.enable = true;
  };
}
