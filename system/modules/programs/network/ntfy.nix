{config, ...}: {
  services.ntfy-sh = {
    enable = true;
    settings = {
      listen-http = ":2586";
      base-url = "http://${config.networking.hostName}:2586";
      # forwards poll_request to ntfy.sh so APNs can wake the iOS app (see docs.ntfy.sh known-issues)
      upstream-base-url = "https://ntfy.sh";

      auth-default-access = "deny-all";
      auth-users = [
        "or1on:$2a$10$RGXSUT9QRK4pWSR45hxj/.xf45AqfQ0EuF6wiU/FOu2xxtrmmUavu:user"
      ];
      auth-access =
        ["or1on:*:ro"]
        ++ map (topic: "*:${topic}:wo") ["ssh" "agents" "notifications" "nyx"];
    };
  };

  networking.firewall.interfaces."tailscale0".allowedTCPPorts = [2586];
}
