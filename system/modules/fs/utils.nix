{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    gparted
    ntfs3g
    exfat
  ];
}
