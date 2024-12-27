{inputs, system, ...}: {
  home.packages = [
    inputs.ghostty.packages.${system}.default
  ];
}
