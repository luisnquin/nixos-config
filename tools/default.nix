{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    (callPackage ./nyx {})
  ];
}
