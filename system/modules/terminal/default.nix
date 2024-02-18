{
  pkgs,
  host,
  ...
}: {
  imports = [
    ./starship.nix
    ./aliases.nix
    ./zsh.nix
  ];

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  environment = {
    interactiveShellInit = builtins.readFile (builtins.path {
      name = "${host.name}-system-shrc-script";
      path = ./dots/.shrc;
    });

    variables.EDITOR = "nano";

    shells = [pkgs.zsh];
  };
}
