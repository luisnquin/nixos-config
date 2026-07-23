{
  inputs,
  lib,
  pkgs,
  system,
  ...
}: let
  tmuxNativeKeybinds = lib.pipe (builtins.readFile ./mods/tmux-native-keybinds.conf) [
    (lib.splitString "\n")
    (builtins.filter (line: line != ""))
    (map (lib.removePrefix "keybind = "))
  ];
in {
  shared.ghostty = {
    enable = true;
    # GTK 4.20 stopped falling back to built-in dead-key/compose handling on
    # Wayland when no input method framework is present, which breaks accents
    # (´ ^ ¨ ~) in GTK4 terminals. Force the legacy "simple" IM module so dead
    # keys compose again. See https://github.com/ghostty-org/ghostty/discussions/8899
    package = inputs.ghostty.packages.${system}.default.overrideAttrs (old: {
      patches =
        (old.patches or [])
        ++ [
          ./patches/agent-tab-progress.patch
          ./patches/agent-tab-icons.patch
          ./patches/tmux-osc7-passthrough.patch
        ];

      preFixup =
        (old.preFixup or "")
        + ''
          gappsWrapperArgs+=(--set-default GTK_IM_MODULE simple)
        '';
    });
    settings = {
      # Every new surface runs tmux; windows launched with -e (herdr) override it.
      command = lib.getExe pkgs.tmux;
      font-size = "10.8";
      gtk-titlebar = "false";
      window-show-tab-bar = "never";
    };
  };

  programs.ghostty.settings.keybind = tmuxNativeKeybinds;
}
