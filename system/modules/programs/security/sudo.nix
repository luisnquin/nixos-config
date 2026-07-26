{
  security.sudo = {
    enable = true;
    execWheelOnly = true;
    wheelNeedsPassword = true;

    configFile = ''
      Defaults 	insults
    '';
  };
}
