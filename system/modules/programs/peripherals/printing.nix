{pkgs, ...}: {
  # Just print it: lp <target>
  # Print and spec target: lp -d EPSON_L3110_Series <target>
  services.printing = {
    enable = true;
    drivers = [pkgs.epson-escpr];
  };

  # TODO:
  # printing/laser
  # printing/3d

  hardware.printers.ensurePrinters = [
    {
      name = "EPSON_L3110_Series";
      description = "That shitty printer";
      location = "Outsource";
      deviceUri = "usb://EPSON/L3110%20Series?serial=583634353439353376&interface=1";
      model = "epson-inkjet-printer-escpr/Epson-L3110_Series-epson-escpr-en.ppd";
      ppdOptions = {
        PageSize = "A4";
        ColorModel = "RGB";
      };
    }
  ];

  environment.systemPackages = [
    # Tests nozzles: sudo escputil -n -P EPSON_L3110_Series -u
    # Head cleaning: sudo escputil -c -P EPSON_L3110_Series -u
    pkgs.gutenprint
  ];

  services.saned.enable = true;

  hardware.sane = {
    enable = true;
    # List devices: scanimage -L
    # Proceed: scanimage -d epson2:libusb:003:006 --mode Color --resolution 300 --format=png > <name>.png
    extraBackends = [pkgs.epkowa];
  };
}
