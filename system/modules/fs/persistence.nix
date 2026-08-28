{
  config,
  user,
  ...
}: {
  preservation = {
    enable = true;

    preserveAt."/persist" = {
      directories = [
        "/var/log"
        "/var/lib/systemd"
        "/var/lib/bluetooth"
        "/var/lib/caddy"
        "/var/lib/docker"
        "/var/lib/tailscale"
        "/var/lib/NetworkManager"
        "/var/lib/alsa"
        "/var/lib/chrony"
        "/var/lib/fail2ban"
        "/var/lib/flatpak"
        "/var/lib/libvirt"
        "/var/lib/power-profiles-daemon"
        "/var/lib/sshguard"
        "/var/lib/private"
        "/root"
        {
          # the uid/gid maps; without them activation reallocates ids and every
          # persisted file ends up owned by a stranger
          directory = "/var/lib/nixos";
          inInitrd = true;
        }
        {
          directory = "/home/${user.alias}";
          inherit (config.users.users.${user.alias}) group;
          user = user.alias;
          mode = "0700";
        }
      ];

      files = [
        {
          # read by sops-nix during activation, which runs before switch-root
          file = "/etc/ssh/ssh_host_ed25519_key";
          inInitrd = true;
          configureParent = true;
          mode = "0600";
        }
        {
          file = "/etc/ssh/ssh_host_ed25519_key.pub";
          mode = "0644";
        }
        {
          file = "/etc/ssh/ssh_host_rsa_key";
          mode = "0600";
        }
        {
          file = "/etc/ssh/ssh_host_rsa_key.pub";
          mode = "0644";
        }
        {
          file = "/etc/machine-id";
          how = "symlink";
          inInitrd = true;
          configureParent = true;
        }
      ];
    };
  };

  systemd.services.systemd-machine-id-commit = {
    unitConfig.ConditionPathIsMountPoint = ["" "/persist/etc/machine-id"];
    serviceConfig.ExecStart = ["" "systemd-machine-id-setup --commit --root /persist"];
  };
}
