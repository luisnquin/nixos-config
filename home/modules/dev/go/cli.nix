{pkgs, ...}: {
  programs.go = {
    enable = true;
    package = pkgs.go_1_22;

    goBin = "go/bin";
    goPath = "go";
  };

  xdg.configFile = {
    "go/env".text = builtins.readFile ../../../dots/go/env;
  };
}
