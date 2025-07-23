{
  inputs,
  system,
  ...
}: {
  home.packages = [
    inputs.nixpkgs-spamton.legacyPackages.${system}.spamton-shimeji
  ];

  wayland.windowManager.hyprland.settings.general.windowrule = let
    rules = ["float" "noblur" "nofocus" "noshadow" "noborder"];
    target = "class:com-group_finity-mascot-Main";
  in
    builtins.map (rule: "${rule}, ${target}") rules;
}
