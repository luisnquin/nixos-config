{
  pkgs,
  user,
  libx,
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

      hostKeys = [
        {
          bits = 4096;
          path = "/etc/ssh/ssh_host_rsa_key";
          type = "rsa";
        }
        {
          path = "/etc/ssh/ssh_host_ed25519_key";
          type = "ed25519";
        }
      ];

      settings = {
        PasswordAuthentication = false;
        KbdInteractiveAuthentication = false;
        PermitRootLogin = "no";
        MaxAuthTries = 3;
        AllowUsers = [user.alias];
      };

      extraConfig = ''
        PerSourcePenalties crash:3600s authfail:3600s max:86400s
      '';
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
