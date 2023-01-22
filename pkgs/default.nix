{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    (callPackage ./transg-tui.nix {})
    (callPackage ./minecraft.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./nyx.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
