{pkgs, ...}: let
  tools = with pkgs; [
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
    (pkgs.symlinkJoin {
      name = "lsyncd-wrapper";
      paths = [pkgs.lsyncd];

      buildInputs = [pkgs.makeWrapper];

      postBuild = ''
        wrapProgram "$out/bin/lsyncd" \
          --set PINENTRY_USER_DATA gui
      '';
    })
  ];
in {
  environment.systemPackages = tools ++ base ++ services;
}
