{pkgs, ...}: {
  programs.go = {
    enable = true;
    package = pkgs.go_1_23;

    goBin = "go/bin";
    goPath = "go";
  };

  xdg.configFile = {
    "go/env".text = builtins.readFile ../../../dots/go/env;
  };
}
