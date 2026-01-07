{pkgs, ...}: {
  programs.thunar = {
    enable = true;
    plugins =  [
      pkgs.thunar-volman
    ];
  };

  services = {
    gvfs.enable = true;
    tumbler.enable = true;
  };
}
