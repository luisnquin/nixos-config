{
  inputs,
  system,
  ...
}: {
  home.packages = [
    inputs.bud.packages.${system}.default
  ];
}
