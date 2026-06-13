{pkgs, ...}: {
  home.packages = [pkgs.wl-clipboard];

  programs.zsh.initContent = builtins.readFile (builtins.path {
    name = "iphone-shrc";
    path = ./iphone.sh;
  });
}
