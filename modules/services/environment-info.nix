{
  pkgs,
  config,
  ...
}: let
  owner = import "/etc/nixos/owner.nix";
in {
  systemd.services.fetch-environment-info = {
    enable = true;
    description = "Fetchs information about the current environment";
    serviceConfig = with owner; {
      Type = "oneshot";
      ExecStart = "${pkgs.zsh}/bin/zsh -c 'fetch_environment_info ${geoLocationServiceApiKey} /home/${username}/.cache/environment-info.json ${username}'";
    };

    after = ["successful-ping-to-google.service"];
    wantedBy = ["multi-user.target"];
  };
}
