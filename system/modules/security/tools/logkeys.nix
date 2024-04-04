{
  config,
  pkgsx,
  ...
}: {
  environment.systemPackages = [pkgsx.logkeys];

  security.sudo.extraRules = [
    {
      groups = [config.users.groups."control".name];
      commands =
        builtins.map (command: {
          inherit command;
          options = ["NOPASSWD" "SETENV"];
        }) [
          "/run/current-system/sw/bin/logkeys"
          "/run/current-system/sw/bin/llkk"
          "/run/current-system/sw/bin/llk"
          "${pkgsx.logkeys}/bin/logkeys"
          "${pkgsx.logkeys}/bin/llkk"
          "${pkgsx.logkeys}/bin/llk"
        ];
    }
  ];
}
