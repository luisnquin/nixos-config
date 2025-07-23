{
  inputs,
  system,
  ...
}: {
  home.packages = let
    inherit (inputs.nix-scripts.packages.${system}) sys-brightness sys-sound;
  in [
    sys-brightness
    sys-sound
  ];
}
