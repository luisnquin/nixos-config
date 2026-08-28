{
  config,
  user,
  ...
}: {
  preservation = {
    enable = true;

    preserveAt."/persist" = {
      # preservation emits one tmpfiles rule per entry defaulting to 0755 root:root,
      # and preservation.conf sorts ahead of the nixpkgs rules for the same paths, so
      # an unpinned entry silently relaxes whatever nixpkgs declared
      directories = [
        "/var/log"
        "/var/lib/systemd"
        "/var/lib/NetworkManager"
        "/var/lib/alsa"
        "/var/lib/fail2ban"
        "/var/lib/flatpak"
        "/var/lib/libvirt"
        "/var/lib/power-profiles-daemon"
        "/var/lib/sshguard"
        {
          directory = "/root";
          mode = "0700";
        }
        {
          # DynamicUser state; systemd refuses to start such a unit if this is laxer
          directory = "/var/lib/private";
          mode = "0700";
        }
        {
          directory = "/var/lib/chrony";
          user = "chrony";
          group = "chrony";
          mode = "0750";
        }
        {
          directory = "/var/lib/bluetooth";
          mode = "0700";
        }
        {
          directory = "/var/lib/tailscale";
          mode = "0700";
        }
        {
          directory = "/var/lib/docker";
          mode = "0710";
        }
        {
          directory = "/var/lib/caddy";
          user = "caddy";
          group = "caddy";
        }
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
