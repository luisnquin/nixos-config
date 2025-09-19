{
  pkgs-extra,
  config,
  ...
}: {
  environment.systemPackages = [pkgs-extra.logkeys];

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
          "${pkgs-extra.logkeys}/bin/logkeys"
          "${pkgs-extra.logkeys}/bin/llkk"
          "${pkgs-extra.logkeys}/bin/llk"
        ];
    }
  ];
}
