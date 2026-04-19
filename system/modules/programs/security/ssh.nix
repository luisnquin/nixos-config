{
  pkgs,
  user,
  libx,
  lib,
  ...
}: {
  programs.ssh.extraConfig = ''
    Host mac
      HostName dyx.local
  '';

  environment = {
    systemPackages = with pkgs; [
      fast-ssh
    ];

    shellAliases = {
      fssh = "fast-ssh";
    };
  };

  services = {
    openssh = {
      enable = true;
      banner = libx.base64.decode "SXQncyB0cnVlLCB5b3UgY2FuIG5ldmVyIGVhdCBhIHBldCB5b3UgbmFtZQ==";
      ports = [
        357
      ];
      openFirewall = true;

      settings = {
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
          "dyx.local" = [
            {
              type = "ed25519";
              key = "AAAAC3NzaC1lZDI1NTE5AAAAIFlkqZgum152rLBvU5STLmN1Jvao+iJpE0vKdli/QaPc";
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
