{
  config,
  pkgs,
  ...
}: {
  programs.go = {
    enable = true;
    package = pkgs.go;

    env = {
      GOPRIVATE = [
        "github.com/chanchitaapp"
        "github.com/0xc000022070"
      ];
      GOBIN = "go/bin";
      GOPATH = "${config.home.homeDirectory}/go";
      GO111MODULE = "on";
      CGO_ENABLED = "0";
    };
  };
}
