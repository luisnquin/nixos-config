{
  # https://github.com/dmedvinsky/gsimplecal/blob/master/src/Config.cpp
  xdg.configFile."gsimplecal/config".text = ''
    show_calendar = 1
    show_timezones = 1
    mark_today = 1
    show_week_numbers = 1
    close_on_unfocus = 0
    clock_format = %a %d %b %H:%M
    force_lang = en_US.utf8t
    mainwindow_decorated = 1
    mainwindow_keep_above = 1
    mainwindow_sticky = 0
    mainwindow_skip_taskbar = 1
    mainwindow_resizable = 0
    mainwindow_position = center # I think that this option is useless in hyprland
    mainwindow_xoffset = 250
    mainwindow_yoffset = 12
    clock_label = .                    …
    clock_tz = :America/El_Salvador

  '';
}
