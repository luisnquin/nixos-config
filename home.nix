{pkgs, ...}: let
  owner = import "/etc/nixos/owner.nix";
in {
  home-manager.users."${owner.username}" = {
    home = {
      stateVersion = "23.05";
      enableNixpkgsReleaseCheck = true;
    };

    programs = {
      # Let Home Manager install and manage itself.
      home-manager.enable = true;
    };

    xdg.enable = true;
  };
}
