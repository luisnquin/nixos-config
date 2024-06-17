{
  config,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    litecli
    pgcli
    mycli
    sqlc
  ];

  programs.zsh.initExtra = let
    # TODO: try to directly add to the path
    inherit (config.home) homeDirectory;
  in ''
    export PATH="${homeDirectory}/.turso:$PATH"
  '';
}
