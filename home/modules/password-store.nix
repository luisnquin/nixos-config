{
  config,
  pkgs,
  lib,
  ...
}: {
  programs.password-store = {
    enable = true;
    package = pkgs.pass-wayland;
  };

  home.sessionVariables = {
    PASSWORD_STORE_DIR = lib.mkForce (
      let
        inherit (config.home) homeDirectory;
      in "${homeDirectory}/.password-store"
    );
  };
}
