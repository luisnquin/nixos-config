{
  services.mako = {
    enable = true;
    settings = {
      font = "Cascadia Code 9";
      background-color = "#1b1924de";
      text-color = "#cbcacf";

      layer = "overlay";
      anchor = "top-right";
      width = 320;
      height = 130;
      padding = "20";
      margin = "10,20,5";

      border-color = "#1b1924de";
      border-radius = 3;
      border-size = 2;

      default-timeout = 6000;
      progress-color = "over #2e1545";
      max-icon-size = 25;
      icons = true;
      sort = "-time";

      "urgency=low" = {
        border-color = "#444152";
      };

      "urgency=normal" = {
        border-color = "#1b1924de";
      };

      "urgency=high" = {
        border-color = "#ff5555";
        default-timeout = 0;
      };

      "category=mpd" = {
        default-timeout = 2000;
        group-by = "category";
      };
    };
  };
}
