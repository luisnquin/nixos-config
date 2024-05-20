{pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    dfeet
  ];

  services.dbus = {
    implementation = "dbus";
  };
}
