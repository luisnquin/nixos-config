pkgs: {
  herdr-autoname = pkgs.callPackage ./herdr-autoname {};
  phone = pkgs.callPackage ./phone {};
  setup = pkgs.callPackage ./setup {};
  voice-gateway = pkgs.callPackage ./voice-gateway {};
}
