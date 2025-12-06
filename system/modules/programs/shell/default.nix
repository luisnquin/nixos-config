{
  pkgs,
  host,
  ...
}: {
  shared = {
    aliases.enable = true;
    zsh.enable = true;
  };

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  environment = {
    interactiveShellInit = builtins.readFile (builtins.path {
      name = "${host.name}-system-shrc-script";
      path = ./.shrc;
    });

    variables.EDITOR = "nano";

    shells = [pkgs.zsh];
  };
}
