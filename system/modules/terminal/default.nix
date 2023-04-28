{
  config,
  pkgs,
  ...
}: {
  imports = [
    ./cool-retro-term.nix
    ./starship.nix
    ./aliases.nix
    ./nushell.nix
    ./tmux.nix
    ./zsh.nix
  ];

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  environment = {
    interactiveShellInit = builtins.readFile ../../dots/.shrc;

    variables.EDITOR = "nano";

    shells = [pkgs.zsh];
  };
}
