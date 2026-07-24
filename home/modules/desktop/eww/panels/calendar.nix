{...}: {
  yuck = ''
    (defwindow calendar
      :monitor 0
      :geometry (geometry
        :x "50%"
        :y "35px"
        :width "270px"
        :anchor "top center")
      :stacking "overlay"
      :focusable false
      (cal-widget))

    (defwidget cal-widget []
      (box :class "cal-box" :orientation "v" :space-evenly false
        (calendar :class "calendar"
                  :show-week-numbers false)))
  '';
}
