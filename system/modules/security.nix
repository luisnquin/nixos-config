{
  user,
  pkgs,
  ...
}: {
  security = {
    sudo = {
      enable = true;
      wheelNeedsPassword = true;

      configFile = ''
        Defaults 	insults
      '';
    };

    # we don't care about if we're using DE or WM
    polkit.enable = true;
  };

  users = {
    defaultUserShell = pkgs.zsh;
    # mutableUsers = false;
    motd = ''Confront [hi]story'';

    users = {
      ${user.alias} = {
        isNormalUser = true;
        home = ''/home/${user.alias}/'';
        # Used by desktop manager
        description = ''${user.alias} 🍂'';
        shell = pkgs.zsh;
        hashedPassword = null;
        # ❄️

        extraGroups = [
          "networkmanager"
          "docker"
          "wheel"
        ];
      };
    };

    extraGroups = {
      vboxusers.members = ["${user.alias}"];
    };
  };

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [20 80 443 8088];
    allowPing = false;
  };

  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };

  services = {
    gnome.gnome-keyring.enable = true;

    openssh = {
      enable = true;
      banner = "plz let me in";

      settings = {
        # Everything in pascal case: https://github.com/NixOS/nixpkgs/issues/234683
        PasswordAuthentication = true;
      };

      knownHosts = let
        primaryPublicKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJcSOpun+OjJng87LUXArDX3y2LLts7pOpfyCC1Mygew luisnquin@rat";
      in {
        "https://github.com" = {
          publicKey = primaryPublicKey;
        };
        "https://gitlab.com" = {
          publicKey = primaryPublicKey;
        };
      };
    };
  };
}
