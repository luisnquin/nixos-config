{
  imports = [
    ./audio.nix
    ./battery.nix
    ./graphics.nix
    ./outage.nix
    ./usb.nix
  ];

  services.fwupd.enable = true;
}
