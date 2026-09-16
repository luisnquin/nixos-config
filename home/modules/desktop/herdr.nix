{
  config,
  lib,
  pkgs,
  ...
}: let
  herdrSession = pkgs.writeShellScript "herdr-scratchpad-session" ''
    export ZDOTDIR=${lib.escapeShellArg config.programs.zsh.dotDir}
    exec ${lib.getExe pkgs.herdr} --session hub
  '';

  drop = import ./hyprland/drop.nix {inherit pkgs lib;} {
    name = "ghostty-herdr";
    class = "ghostty.herdr";
    command = "${lib.getExe config.programs.ghostty.package} --class=ghostty.herdr --keybind=clear --keybind=ctrl+shift+c=copy_to_clipboard --keybind=ctrl+shift+v=paste_from_clipboard -e ${herdrSession}";
  };

  inherit (lib.generators) mkLuaInline;
in {
  wayland.windowManager.hyprland.settings = lib.mkIf config.wayland.windowManager.hyprland.enable {
    window_rule = [drop.windowRule];

    bind = [
      {_args = ["SUPER + J" drop.toggle];}
    ];

    on = [
      {
        _args = [
          "hyprland.start"
          (mkLuaInline ''
            function()
              ${drop.startupBody}
            end
          '')
        ];
      }
    ];
  };
}
