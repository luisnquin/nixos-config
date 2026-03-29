{pkgs, ...}: let
  tools = with pkgs; [
    wireshark
    nload
    netcat
  ];

  base = with pkgs; [
    inetutils
    iptables
    wirelesstools
  ];

  services = with pkgs; [
    cloudflared # https://developers.cloudflare.com/ssl/edge-certificates/additional-options/total-tls/error-messages/#active-domains
    lsyncd
  ];
in {
  environment.systemPackages = tools ++ base ++ services;
}
