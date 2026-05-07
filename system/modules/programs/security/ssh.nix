{
  config,
  libx,
  pkgs,
  user,
  lib,
  ...
}: {
  programs.ssh.extraConfig = ''
    Host mac-local
      HostName rose.local
  '';

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

          ${libx.comms.mkNtfy config.services.ntfy-sh.settings.base-url {
          topic = "ssh";
          title = config.networking.hostName;
          message = "New SSH connection!";
        }}
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
      openFirewall = true;

      settings = {
        Banner = "/etc/ssh/ssh-banner";
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
        AuthenticationMethods = "publickey";
        PubkeyAuthentication = "yes";
        ChallengeResponseAuthentication = "no";
        PermitRootLogin = "no";
        MaxAuthTries = 3;
        AllowUsers = [user.alias];
        X11Forwarding = false;
        ClientAliveCountMax = 3;
        ClientAliveInterval = 60;
      };

      extraConfig = ''
        PerSourcePenalties crash:3600s authfail:3600s max:86400s
      '';

      knownHosts = let
        mkPub = host: key: {
          "${host}-${key.type}" = {
            hostNames = [host];
            publicKey = "ssh-${key.type} ${key.key}";
          };
        };

        # thank you isabel parker
        mkPubs = host: keys: lib.foldl' (acc: key: acc // mkPub host key) {} keys;
      in
        lib.concatMapAttrs mkPubs {
          "github.com" = [
            {
              type = "ed25519";
              key = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl";
            }
            {
              type = "rsa";
              key = "AAAAB3NzaC1yc2EAAAADAQABAAABgQCj7ndNxQowgcQnjshcLrqPEiiphnt+VTTvDP6mHBL9j1aNUkY4Ue1gvwnGLVlOhGeYrnZaMgRK6+PKCUXaDbC7qtbW8gIkhL7aGCsOr/C56SJMy/BCZfxd1nWzAOxSDPgVsmerOBYfNqltV9/hWCqBywINIR+5dIg6JTJ72pcEpEjcYgXkE2YEFXV1JHnsKgbLWNlhScqb2UmyRkQyytRLtL+38TGxkxCflmO+5Z8CSSNY7GidjMIZ7Q4zMjA2n1nGrlTDkzwDCsw+wqFPGQA179cnfGWOWRVruj16z6XyvxvjJwbz0wQZ75XK5tKSb7FNyeIEs4TT4jk+S4dhPeAUC5y+bDYirYgM4GC7uEnztnZyaVWQ7B381AK4Qdrwt51ZqExKbQpTUNn+EjqoTwvqNj4kqx5QUCI0ThS/YkOxJCXmPUWZbhjpCg56i+2aB6CmK2JGhn57K5mj0MNdBXA4/WnwH6XoPWJzK5Nyu2zB3nAZp+S5hpQs+p1vN1/wsjk=";
            }
          ];
          "git.encore.dev" = [
            {
              type = "ed25519";
              key = "AAAAC3NzaC1lZDI1NTE5AAAAIHnYnozNaCKhAsIL0vWc4NEW1XoPXS98pMFivH+PRzwr";
            }
          ];
        };
    };

    endlessh = {
      enable = true;
      port = 22;
      openFirewall = true;
    };

    fail2ban = {
      enable = true;
      maxretry = 5;
      bantime = "24h";
      bantime-increment = {
        enable = true;
        formula = "ban.Time * math.exp(float(ban.Count+1)*banFactor)/math.exp(1*banFactor)";
        maxtime = "192h";
        overalljails = true;
      };
    };
  };
}
