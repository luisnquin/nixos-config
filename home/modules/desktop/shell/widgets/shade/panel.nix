{
  name,
  lib,
  pkgs,
  ...
}: let
  # eww 0.6 exposes no time magic variable, so the almanac heading comes off a
  # poll like every other datum in the shell.
  shadeDate = pkgs.writeShellApplication {
    name = "eww-shade-date";
    runtimeInputs = [pkgs.coreutils];
    text = ''
      date '+{"weekday":"%A","full":"%B %-d, %Y"}'
    '';
  };
in {
  yuck = ''
    (defpoll shade_date :interval "60s"
      :initial '{"weekday":"","full":""}'
      `${lib.getExe shadeDate}`)

    ; With a "top center" anchor eww reads :x as where the window's own centre
    ; lands, so 50% is dead centre whatever the shade ends up measuring.
    (defwindow ${name}
      :monitor 0
      :geometry (geometry
        :x "50%"
        :y "35px"
        :anchor "top center")
      :stacking "overlay"
      :focusable false
      (shade-widget))

    (defwidget shade-widget []
      (box :class "shade-box" :orientation "v" :space-evenly false :spacing 8
           :width 290
        (box :class "shade-heading" :orientation "v" :space-evenly false
          (label :class "shade-weekday" :halign "start" :text {shade_date.weekday})
          (label :class "shade-full-date" :halign "start" :text {shade_date.full}))
        (calendar :class "calendar" :show-week-numbers false)))
  '';
}
