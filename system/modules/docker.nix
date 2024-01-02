{pkgs, ...}: {
  networking.firewall.trustedInterfaces = ["docker0"];

  virtualisation.docker = {
    enable = true;
    enableNvidia = true;
    extraOptions = "--default-runtime=nvidia";

    autoPrune = {
      enable = true;
      dates = "daily";
    };
  };

  environment = {
    systemPackages = with pkgs; [
      docker
      lazydocker

      # hadolint # Dockerfile linter
    ];

    shellAliases = {
      dka = "docker kill $(docker ps -qa) 2> /dev/null";
      dra = "docker rm -f $(docker ps -qa) 2> /dev/null";
      dkra = "dka; dra";
      dria = "docker rmi -f $(docker image ls -qa)";
      dils = "docker image ls";
      dcp = "docker cp";
      dps = "docker ps -a";
      dl = "docker logs";
      ld = "lazydocker";
    };
  };
}
