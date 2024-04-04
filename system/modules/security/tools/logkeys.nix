{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = [pkgs.logkeys];

  security.sudo.extraRules = [
    {
      groups = [config.users.groups."control".name];
      commands =
        builtins.map (command: {
          inherit command;
          options = ["NOPASSWD" "SETENV"];
        }) [
          "/run/current-system/sw/bin/logkeys"
          "/run/current-system/sw/bin/llk"
          "/run/current-system/sw/bin/llkk"
        ];
    }
  ];
}
