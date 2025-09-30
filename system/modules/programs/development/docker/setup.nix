{user, ...}: {
  networking.firewall.trustedInterfaces = ["docker0"];
  hardware.nvidia-container-toolkit.enable = true;
  users.users.${user.alias}.extraGroups = ["docker"];

  virtualisation.docker = {
    enable = true;
    daemon.settings = {
      "builder" = {
        "gc" = {
          "enabled" = true;
          "defaultKeepStorage" = "10GB";
          "policy" = [
            {
              "keepStorage" = "8GB";
              "filter" = ["unused-for=1440h"];
            }
            {
              "keepStorage" = "15GB";
              "filter" = ["unused-for=2200h"];
            }
            {
              "keepStorage" = "20GB";
              "all" = true;
            }
          ];
        };
      };
      "max-concurrent-downloads" = 5;
      "max-concurrent-uploads" = 5;
      "max-download-attempts" = 3;
    };
    autoPrune = {
      enable = true;
      dates = "daily";
      flags = [
        "--all"
        "--force"
        "--filter=until=720h" # 30d
      ];
    };
  };
}
