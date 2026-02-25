{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    coreutils
    binutils
    tree
    lsof
    wget
    eza # ls replacement
  ];

  boot.binfmt.emulatedSystems = ["aarch64-linux"];
}
