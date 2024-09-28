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
      then pkgs.pass-wayland
      else pkgs.pass;
  };

  home.sessionVariables = {
    PASSWORD_STORE_DIR = lib.mkForce (
      let
        inherit (config.home) homeDirectory;
      in "${homeDirectory}/.password-store" # this could be your attach vector!
    );
  };
}
