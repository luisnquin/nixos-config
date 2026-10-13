{pkgs, ...}: {
  environment.systemPackages = [pkgs.podman-compose];

  hardware.nvidia-container-toolkit.enable = true;

  virtualisation.containers.registries.settings.unqualified-search-registries = ["docker.io"];

  virtualisation.podman = {
    enable = true;
    dockerCompat = true;
  };
}
