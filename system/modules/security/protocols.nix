{
  pkgs,
  user,
  ...
}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };

  environment = {
    systemPackages = with pkgs; [
      fast-ssh
    ];

    shellAliases = {
      fssh = "fast-ssh";
    };
  };

  services.openssh = {
    enable = true;
    banner = pkgs.libx.base64.decode "SXQncyB0cnVlLCB5b3UgY2FuIG5ldmVyIGVhdCBhIHBldCB5b3UgbmFtZQ==";
    ports = [
      357
    ];
    openFirewall = false;

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
      {
        path = "/home/${user.alias}/.ssh/id_ed25519";
        type = "ed25519";
      }
      {
        path = "/home/${user.alias}/.ssh/id_ed25520";
        type = "ed25519";
      }
    ];

    # https://github.com/NixOS/nixpkgs/issues/234683
    settings = {
      PasswordAuthentication = true;
    };
  };
}
