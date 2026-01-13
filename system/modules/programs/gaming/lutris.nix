{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    (lutris.override {
      extraPkgs = pkgs: [
        (wine.override {wineBuild = "wine64";})
      ];
    })
  ];
}
