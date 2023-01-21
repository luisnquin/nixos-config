{
  pkgs,
  config,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  systemd.services.fetchEnvironmentInfo = {
    enable = true;
    description = "Fetchs information about the current environment";
    serviceConfig = with owner; {
      Type = "oneshot"; # TODO: create unit that pings to depend on
      ExecStart = "${pkgs.zsh}/bin/zsh -c 'until ping -c1 google.com; do sleep 1; done; fetch_environment_info ${geoLocationServiceApiKey} /home/${username}/.cache/environment-info.json ${username}'";
    };

    after = ["network-online.target"];
    wants = ["network-online.target"];
    wantedBy = ["multi-user.target"];
  };
}
