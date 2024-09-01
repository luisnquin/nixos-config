{
  config,
  pkgs,
  ...
}: let
  mysql-client = (
    pkgs.linkFarm "mysql-client" [
      {
        name = "bin/mysql";
        path = "${pkgs.mariadb}/bin/mysql";
      }
    ]
  );
in {
  home.packages = with pkgs; [
    mysql-client # I prefer containers for server stuff
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
