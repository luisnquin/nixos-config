{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    (lutris.override {
      extraPkgs = _pkgs: [
        (wine.override {wineBuild = "wine64";})
      ];
    })
  ];
}
