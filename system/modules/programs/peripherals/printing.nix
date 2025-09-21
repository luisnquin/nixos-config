{pkgs, ...}: {
  services.printing = {
    enable = true;
    drivers = [pkgs.epson-escpr];
  };

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
}
