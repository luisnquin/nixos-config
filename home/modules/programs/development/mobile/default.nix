{
  imports = [
    ./options.nix
    ./android.nix
    ./avds.nix
  ];

  programs.phone = {
    enable = true;
    hosts.rose.clone = false;
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
