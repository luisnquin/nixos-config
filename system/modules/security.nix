{
  user,
  pkgs,
  libx,
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
    motd = libx.base64.decode "Q29uZnJvbnQgW2hpXXN0b3J5";
    defaultUserShell = pkgs.zsh;

    users.${user.alias} = {
      description = ''Dorian 🍂'';

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
      banner = libx.base64.decode "SXQncyB0cnVlLCB5b3UgY2FuIG5ldmVyIGVhdCBhIHBldCB5b3UgbmFtZQ==";

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

