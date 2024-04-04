{libx, ...}: {
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };

  services.openssh = {
    enable = true;
    banner = libx.base64.decode "SXQncyB0cnVlLCB5b3UgY2FuIG5ldmVyIGVhdCBhIHBldCB5b3UgbmFtZQ==";

    # https://github.com/NixOS/nixpkgs/issues/234683
    settings = {
      PasswordAuthentication = true;
    };
  };
}
