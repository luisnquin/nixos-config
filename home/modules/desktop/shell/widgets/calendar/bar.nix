{toggle, ...}: {
  "clock" = {
    interval = 60;
    format = " {:%H:%M}";
    tooltip = true;
    tooltip-format = "{:%A, %B %d %Y}";
    on-click = toggle;
  };
}
