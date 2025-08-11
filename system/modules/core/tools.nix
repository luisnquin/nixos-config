{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    coreutils
    binutils
    tree
    lsof
    wget
    eza # ls replacement
  ];
}
