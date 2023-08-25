{
  host,
  pkgs,
  ...
}: {
  boot = with pkgs; {
    loader = {
      efi.canTouchEfiVariables = true;
      timeout = 15;

      grub = let
        falloutTheme =
          fetchFromGitHub
          {
            owner = "shvchk";
            repo = "fallout-grub-theme";
            rev = "80734103d0b48d724f0928e8082b6755bd3b2078";
            sha256 = "sha256-7kvLfD6Nz4cEMrmCA9yq4enyqVyqiTkVZV5y4RyUatU=";
          };
      in {
        gfxmodeBios = host.resolution;
        gfxmodeEfi = host.resolution;
        theme = falloutTheme;

        enable = true;
        device = "nodev";
        useOSProber = true;
        efiSupport = true;
        forceInstall = false;
      };
    };

    kernelPackages = linuxPackages_latest;
    supportedFilesystems = ["ntfs"];
    tmp.cleanOnBoot = true;
  };
}
