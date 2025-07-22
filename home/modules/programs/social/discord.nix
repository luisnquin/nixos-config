{pkgs, ...}: {
  home.packages = [
    (pkgs.discord-canary.override {
      withVencord = true;
      withOpenASAR = true;
      nss = pkgs.nss_latest;
    })
  ];
}
