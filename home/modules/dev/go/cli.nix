{pkgs, ...}: {
  programs.go = {
    enable = true;
    package = pkgs.go_1_23;

    goPrivate = [
      "github.com/chanchitaapp"
      "github.com/0xc000022070"
    ];

    goBin = "go/bin";
    goPath = "go";
  };

  xdg.configFile = {
    "go/env".text = builtins.readFile ../../../dots/go/env;
  };
}
