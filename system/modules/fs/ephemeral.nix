{
  fileSystems = {
    "/" = {
      device = "none";
      fsType = "tmpfs";
      options = ["defaults" "size=8G" "mode=755"];
    };

    "/persist".neededForBoot = true;

    "/nix" = {
      device = "/persist/nix";
      fsType = "none";
      options = ["bind"];
      depends = ["/persist"];
    };
  };
}
