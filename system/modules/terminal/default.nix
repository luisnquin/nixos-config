{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./starship.nix
    ./aliases.nix
    ./tmux.nix
    ./zsh.nix
  ];

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  environment = {
    interactiveShellInit = builtins.readFile ../../dots/.shrc;

    systemPackages = with pkgs; [
      cached-nix-shell
    ];

    variables = {
      # The other related config only apply to the build
      NIXPKGS_ALLOW_UNFREE = "1";
      EDITOR = "nano";
    };

    shells = [pkgs.zsh];
  };
}
