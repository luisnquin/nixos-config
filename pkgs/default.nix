{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    (callPackage ./transg-tui.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
