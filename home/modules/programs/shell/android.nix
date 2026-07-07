{pkgs, ...}: {
  home.packages = [pkgs.wl-clipboard];

  programs.zsh.initContent = builtins.readFile (builtins.path {
    name = "android-shrc";
    path = ./android.sh;
  });
}
