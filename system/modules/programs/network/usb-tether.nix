# Routes the mobile-nixos phone's rndis subnet out through this host.
{...}: let
  # Imposed rather than observed: the kernel name encodes the usb socket, so it
  # changes on a replug.
  interface = "usbtether0";

  # 18d1:4ee7 is Google's generic android rndis pair, which any tethering phone
  # answers to; the model string is what narrows the match to one device.
  gadget = {
    vendorId = "18d1";
    productId = "4ee7";
    model = "xiaomi-dandelion";
  };

  # Declared by the device, not here. A mismatch costs it the internet with
  # nothing logged on either side.
  subnet = "172.16.42.0/24";

  hostAddress = "172.16.42.2";
in {
  systemd.network.links."10-${interface}" = {
    matchConfig.Property = [
      "ID_VENDOR_ID=${gadget.vendorId}"
      "ID_MODEL_ID=${gadget.productId}"
      "ID_MODEL=${gadget.model}"
    ];
    linkConfig.Name = interface;
  };

  # Static rather than DHCP: the device's dnsmasq is authoritative and sends
  # itself as a gateway, so NetworkManager default-routes this host through the
  # phone.
  networking.networkmanager.unmanaged = ["interface-name:${interface}"];
  networking.interfaces.${interface}.ipv4.addresses = [
    {
      address = hostAddress;
      prefixLength = 24;
    }
  ];

  networking.nat = {
    enable = true;
    internalIPs = [subnet];

    # Unpinned. Naming an uplink stamps `-o <iface>` on both the masquerade and
    # the forward ACCEPT, and this host has two default routes.
    externalInterface = null;
  };

  networking.firewall.trustedInterfaces = [interface];

  services.avahi.allowInterfaces = [interface];
}
