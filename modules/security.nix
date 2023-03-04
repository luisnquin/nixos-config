{
  config,
  pkgs,
  ...
}: let
  owner = import ../owner.nix;
in {
  security = {
    sudo = {
      enable = true;
      wheelNeedsPassword = true;
    };
  };

  users = {
    motd = "It's a good moment to tell you that this will be a great day for you 🌇";

    users = with owner; {
      ${username} = {
        isNormalUser = true;
        home = ''/home/${username}/'';
        # Used by desktop manager
        description = ''${username} 🌂'';
        shell = pkgs.zsh;
        hashedPassword = null;
        # ❄️

        extraGroups = [
          "wheel"
          "docker"
        ];
      };
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
      banner = "Hiiii how'r u, plz let me in";
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
