# A hyprdrop scratchpad: spawned hidden at startup, toggled onto the active
# workspace, and always dropped back at the geometry its window rule handed it.
{
  pkgs,
  lib,
}: let
  toLua = lib.generators.toLua {};

  inherit (lib.generators) mkLuaInline;

  hyprdropCmd = lib.getExe pkgs.hyprdrop;
in
  {
    name,
    class,
    command,
    width ? 1280,
    height ? 720,
  }: let
    classRegex = "^${lib.escapeRegex class}$";
    selector = toLua "class:${classRegex}";
    toggleCommand = "${hyprdropCmd} --solo -i ${class} ${lib.escapeShellArg command}";
  in {
    windowRule = {
      inherit name;
      match = {class = classRegex;};
      float = true;
      size = "${toString width} ${toString height}";
      center = true;
    };

    startupBody = ''hl.exec_cmd(${toLua "[workspace special:hyprdrop silent] ${command}"})'';

    # Window rules only fire on map, so a window dragged while out keeps its
    # geometry forever. Replay the rule while the drop is still stowed: it stays
    # freely movable once out, and comes back centered on the next toggle.
    toggle = mkLuaInline ''
      function()
        local win = hl.get_window(${selector})

        if win ~= nil and win.workspace ~= nil and win.workspace.special then
          if not win.floating then
            hl.dispatch(hl.dsp.window.float({ window = ${selector} }))
          end

          hl.dispatch(hl.dsp.window.resize({ window = ${selector}, x = ${toString width}, y = ${toString height}, relative = false }))
          hl.dispatch(hl.dsp.window.center({ window = ${selector} }))
        end

        hl.exec_cmd(${toLua toggleCommand})
      end
    '';
  }
