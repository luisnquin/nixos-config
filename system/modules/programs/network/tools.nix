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
    cloudflared
    lsyncd
  ];
in {
  environment.systemPackages = tools ++ base ++ services;
}
