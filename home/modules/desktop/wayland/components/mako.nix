{
  services.mako = {
    enable = true;

    font = "Cascadia Code 9";
    backgroundColor = "#1b1924de";
    textColor = "#cbcacf";

    layer = "overlay";
    anchor = "top-right";
    width = 320;
    height = 130;
    padding = "20";
    margin = "5";

    borderColor = "#1b1924de";
    borderRadius = 3;
    borderSize = 2;

    defaultTimeout = 6000;
    progressColor = "over #2e1545";
    maxIconSize = 25;
    icons = true;
    sort = "-time";

    extraConfig = ''
      [urgency=low]
      border-color=#444152

      [urgency=normal]
      border-color=#1b1924de

      [urgency=high]
      border-color=#ff5555
      default-timeout=0

      [category=mpd]
      default-timeout=2000
      group-by=category
    '';
  };
}
