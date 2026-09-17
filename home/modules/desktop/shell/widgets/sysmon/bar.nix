{toggle, ...}: {
  "group/sysmon" = {
    orientation = "vertical";
    modules = ["cpu" "memory"];
  };

  "cpu" = {
    "interval" = 1;
    "format" = "󰍛 {usage}%";
    "on-click" = toggle;
  };

  "memory" = {
    "interval" = 1;
    "format" = " {percentage}%";
    "states" = {
      "warning" = 80;
      "critical" = 95;
    };
    "on-click" = toggle;
  };
}
