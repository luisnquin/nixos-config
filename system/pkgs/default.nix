{
  config,
  pkgs,
  ...
}: {
  environment.systemPackages = with pkgs; [
    (callPackage ./panicparse.nix {})
    (callPackage ./transg-tui.nix {})
    (callPackage ./minecraft.nix {})
    (callPackage ./pg-ping.nix {})
    (callPackage ./npkill.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./nyx.nix {})
    (callPackage ./no.nix {})
    # (callPackage ./nao.nix {})
  ];
}
