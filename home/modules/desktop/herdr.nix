{
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib.generators) mkLuaInline;

  herdrSession = pkgs.writeShellScript "herdr-scratchpad-session" ''
    export ZDOTDIR=${lib.escapeShellArg config.programs.zsh.dotDir}
    exec ${lib.getExe pkgs.herdr} --session hub
  '';

  launchCommand = "${lib.getExe config.programs.ghostty.package} --class=ghostty.herdr --keybind=clear --keybind=ctrl+shift+c=copy_to_clipboard --keybind=ctrl+shift+v=paste_from_clipboard -e ${herdrSession}";
  toggleCommand = "${lib.getExe pkgs.hyprdrop} --solo -i ghostty.herdr ${lib.escapeShellArg launchCommand}";
  toLua = lib.generators.toLua {};
in {
  wayland.windowManager.hyprland.settings = lib.mkIf config.wayland.windowManager.hyprland.enable {
    window_rule = [
      {
        name = "ghostty-herdr";
        match = {class = "^ghostty\\.herdr$";};
        float = true;
        size = "1280 720";
        center = true;
      }
    ];

    bind = [
      {
        _args = [
          "SUPER + J"
          (mkLuaInline "hl.dsp.exec_cmd(${toLua toggleCommand})")
        ];
      }
    ];

    on = [
      {
        _args = [
          "hyprland.start"
          (mkLuaInline ''
            function()
              hl.exec_cmd(${toLua "[workspace special:hyprdrop silent] ${launchCommand}"})
            end
          '')
        ];
      }
    ];
  };
}
