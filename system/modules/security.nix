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

    polkit.enable = true;
  };

  users = {
    defaultUserShell = pkgs.zsh;
    # mutableUsers = false;
    motd = ''Confront [hi]story'';

    users.${user.alias} = {
      description = ''${user.alias} 🍂'';

      home = ''/home/${user.alias}/'';
      hashedPassword = null;
      isNormalUser = true;

      extraGroups = [
        "networkmanager"
        "docker"
        "wheel"
      ];

      shell = pkgs.zsh;
    };

    extraGroups = {
      vboxusers.members = [user.alias];
    };
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

      # https://github.com/NixOS/nixpkgs/issues/234683
      settings = {
        PasswordAuthentication = true;
      };
    };
  };
}
#
# knownHosts = let
#   primaryPublicKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJcSOpun+OjJng87LUXArDX3y2LLts7pOpfyCC1Mygew luisnquin@rat";
# in {
#   "https://github.com" = {
#     publicKey = primaryPublicKey;
#   };
#   "https://gitlab.com" = {
#     publicKey = primaryPublicKey;
#   };
# };
