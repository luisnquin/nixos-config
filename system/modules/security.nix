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
  };

  users = {
    motd = ''
      Our galaxy consists of some 300 billion stars. Around half are orbited by planets, and it's said that on average, conditions on two of a star's planets are suitable for life.

      No great leap of imagination is needed to believe the universe must be home to a myriad of life forms.
      But what sorts of intelligence would develop on these worlds?
      That's truly beyond our imagination.'';

    users = {
      ${user.alias} = {
        isNormalUser = true;
        home = ''/home/${user.alias}/'';
        # Used by desktop manager
        description = ''${user.alias} 🌂'';
        shell = pkgs.zsh;
        hashedPassword = null;
        # ❄️

        extraGroups = [
          "wheel"
          "docker"
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
      settings.passwordAuthentication = true;

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
