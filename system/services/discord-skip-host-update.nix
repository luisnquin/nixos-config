{
  pkgs,
  user,
  ...
}: {
  systemd.services.discord-skip-host-update = {
    enable = true;
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''
        ${pkgs.bash}/bin/bash -c "DISCORD_DIR=\"/home/\${user.alias}/.config/discord\"; \
              DISCORD_SETTINGS_FILE=\"\$DISCORD_DIR/settings.json\"; \
              if ! test -f \"\$DISCORD_SETTINGS_FILE\"; then \
                  mkdir -p \"\$DISCORD_DIR\" && echo '{\"SKIP_HOST_UPDATE\": true}' > \"\$DISCORD_SETTINGS_FILE\"; \
              else \
                  ${pkgs.jq}/bin/jq -s '.[0] + {\"SKIP_HOST_UPDATE\": true}' \"\$DISCORD_SETTINGS_FILE\" > \"\$DISCORD_SETTINGS_FILE\"; \
              fi"
      '';
    };
  };
}
