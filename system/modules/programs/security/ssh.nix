{
  config,
  inputs,
  libx,
  pkgs,
  user,
  ...
}: let
  sshNotify = libx.notify.ntfy.send {
    host = config.services.ntfy-sh.settings.base-url;
    topic = "ssh";
    title = config.networking.hostName;
    message = "New SSH connection!";
  };
in {
  programs.ssh.extraConfig = ''
    Host rose
      User luisnquin

    Host ori0n ori0n.local
      Port 963
      User luisnquin
  '';

  networking.firewall.interfaces."tailscale0".allowedTCPPorts = config.services.openssh.ports;

  environment = {
    systemPackages = with pkgs; [
      fast-ssh
    ];

    shellAliases = {
      fssh = "fast-ssh";
    };

    interactiveShellInit = ''
      ssh_count() {
        local inbound outbound

        inbound="$(
          who | awk '$2 ~ /^pts\// && $NF ~ /^\(/ { n++ } END { print n+0 }'
        )"

        outbound="$(
          pgrep -u "$USER" -x ssh | wc -l
        )"

        printf "SSH inbound : %s\n" "$inbound"
        printf "SSH outbound: %s\n" "$outbound"
      }

      ssh_unlock() {
        ssh -T git@github.com
      }
    '';
  };

  environment.etc = {
    "ssh/ssh-banner".text = ''
      It's true, you can never eat a pet you name
    '';

    "ssh/sshrc" = {
      mode = "0755";
      text = ''
        #!${pkgs.runtimeShell}

        client_ip="''${SSH_CONNECTION%% *}"
        state_dir="''${XDG_RUNTIME_DIR:-/tmp}/sshrc-ntfy"
        stamp="$state_dir/$client_ip"

        mkdir -p "$state_dir"

        now="$(date +%s)"
        last="$(cat "$stamp" 2>/dev/null || echo 0)"

        if [ "$((now - last))" -ge 10 ]; then
          echo "$now" > "$stamp"

          ${sshNotify}
        fi
      '';
    };
  };

  services = {
    openssh = {
      enable = true;
      ports = [
        357
      ];
      openFirewall = false;

      settings = {
        Banner = "/etc/ssh/ssh-banner";
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
        AuthenticationMethods = "publickey";
        PubkeyAuthentication = "yes";
        ChallengeResponseAuthentication = "no";
        PermitRootLogin = "no";
        MaxAuthTries = 3;
        LoginGraceTime = 20;
        AllowUsers = [user.alias];
        X11Forwarding = false;
        AllowAgentForwarding = false;
        ClientAliveCountMax = 3;
        ClientAliveInterval = 60;
      };

      extraConfig = ''
        PerSourcePenalties crash:3600s authfail:3600s max:86400s
      '';

      knownHosts = inputs.identity.lib.ssh.nixosKnownHosts;
    };

    endlessh = {
      enable = true;
      port = 22;
      openFirewall = true;
    };

    sshguard = {
      enable = true;
      services = ["sshd"];
      attack_threshold = 50;
      blocktime = 86400;
    };
  };
}
