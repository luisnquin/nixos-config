{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    d-spy
  ];

  services.dbus = {
    implementation = "dbus";
  };
}
