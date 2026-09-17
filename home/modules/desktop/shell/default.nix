# The desktop shell: waybar draws the bar, eww draws the panels behind it, and
# widgets/ holds both halves of each readout. eww.nix and waybar.nix are config
# fragments rather than modules — neither is usable without the other, and
# imports cannot carry a config-dependent value like the widget list.
{
  config,
  lib,
  pkgs,
  ...
}: let
  widgets = import ./widgets {inherit config lib pkgs;};

  ctx = {inherit config lib pkgs widgets;};
in
  lib.mkMerge [
    (import ./eww.nix ctx)
    (import ./waybar.nix ctx)
  ]
