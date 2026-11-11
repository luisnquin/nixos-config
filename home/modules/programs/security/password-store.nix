{
  isWayland,
  config,
  pkgs,
  lib,
  ...
}: {
  programs.password-store = {
    enable = true;
    package =
      if isWayland
      then pkgs.pass-wayland.withExtensions (exts: [exts.pass-otp])
      else pkgs.pass.withExtensions (exts: [exts.pass-otp]);
  };

  home.sessionVariables = {
    PASSWORD_STORE_DIR = lib.mkForce (
      let
        inherit (config.home) homeDirectory;
      in "${homeDirectory}/.password-store"
    );
  };
}
