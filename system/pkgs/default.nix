{pkgs, ...}: {
  environment.systemPackages = with pkgs; let
    paths = [
      ../../tools/nyx

      # ./docker-desktop

      ./panicparse.nix
      ./transg-tui.nix
      ./minecraft.nix
      ./pg-ping.nix
      ./npkill.nix
      ./tomato.nix
      ./nao.nix
      ./no.nix
    ];
  in
    lib.lists.forEach paths (path: callPackage path {});
}
