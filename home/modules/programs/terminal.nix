{
  inputs,
  system,
  ...
}: {
  shared = {
    alacritty.enable = true;
    ghostty = {
      enable = true;
      # GTK 4.20 stopped falling back to built-in dead-key/compose handling on
      # Wayland when no input method framework is present, which breaks accents
      # (´ ^ ¨ ~) in GTK4 terminals. Force the legacy "simple" IM module so dead
      # keys compose again. See https://github.com/ghostty-org/ghostty/discussions/8899
      package = inputs.ghostty.packages.${system}.default.overrideAttrs (old: {
        preFixup =
          (old.preFixup or "")
          + ''
            gappsWrapperArgs+=(--set-default GTK_IM_MODULE simple)
          '';
      });
      settings = {
        font-size = "10.8";
      };
    };
  };

  # cursor is trying to autoupdate _._
}
