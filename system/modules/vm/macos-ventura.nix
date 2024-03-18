{nixtheplanet, ...}: {
  services.macos-ventura = {
    enable = true;
    package = nixtheplanet.legacyPackages.x86_64-linux.makeDarwinImage {diskSizeBytes = 60000000000;};
    openFirewall = true;
    # vcnListenAddr = "0.0.0.0";
    autoStart = false;
    dataDir = "/var/lib/nixtheplanet-macos-ventura";
    mem = "5G";
  };
}
