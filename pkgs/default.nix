{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    # (callPackage ./chatgpt-desktop.nix {})
    (callPackage ./transg-tui.nix {})
    (callPackage ./minecraft.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./ai-cli.nix {})
    (callPackage ./nyx.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
