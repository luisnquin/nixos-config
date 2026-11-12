{
  imports = [
    ./options.nix
    ./android.nix
    ./avds.nix
  ];

  programs.phone = {
    enable = true;
    hosts = {
      nyx.enable = true;
      chimera.enable = true;
      rose = {
        enable = true;
        clone = false;
      };
    };
    ledger = "nyx";
    pools = {
      android = ["pixel_7-api36" "pixel_7-api36-b" "pixel_7-api36-c"];
      ios = ["iPhone 17 Pro Max" "iPhone 17 Pro"];
    };
    devices.faraday = {
      kind = "physical";
      pick = "last";
      lease.ttl = "2m";
    };
  };
}
