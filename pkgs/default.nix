{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    (callPackage ./tomato.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
