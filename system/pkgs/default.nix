{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    (callPackage ../../tools/nyx {})
    (callPackage ./panicparse.nix {})
    (callPackage ./transg-tui.nix {})
    (callPackage ./minecraft.nix {})
    (callPackage ./pg-ping.nix {})
    (callPackage ./npkill.nix {})
    (callPackage ./tomato.nix {})
    (callPackage ./no.nix {})
  ];
}
