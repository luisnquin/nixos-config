{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    # (callPackage ./terminalizer.nix {})
    (callPackage ./transg-tui.nix {})
    (callPackage ./minecraft.nix {})
    (callPackage ./ai-cli.nix {})
    (callPackage ./npkill.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./nyx.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
