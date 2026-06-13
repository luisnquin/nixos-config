{
  pkgs,
  ...
}: {
  programs.eww = {
    enable = true;
    package = pkgs.eww;

    systemd = {
      enable = true;
      target = "graphical-session.target";
    };

    yuckConfig = ''
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

    scssConfig = ''
      * {
        font-family: "JetBrainsMono Nerd Font";
        font-size: 10pt;
        font-weight: bold;
        color: #cdd6f4;
      }

      window,
      .background {
        background-color: transparent;
      }

      .cal-box {
        background-color: rgba(29, 16, 58, 0.97);
        border: 1px solid rgba(181, 232, 224, 0.12);
        border-radius: 10px;
        padding: 12px 16px;
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.45);
      }

      calendar {
        background-color: transparent;
        color: #cdd6f4;
        border-radius: 6px;
      }

      calendar.header {
        background-color: transparent;
        color: #e8cef5;
        font-weight: bold;
        padding: 2px 0 6px 0;
      }

      calendar.day-name {
        background-color: transparent;
        color: #b5e8e0;
        font-weight: bold;
        padding: 2px 0;
      }

      calendar.day {
        background-color: transparent;
        color: #cdd6f4;
        border-radius: 4px;
        padding: 1px 3px;
      }

      calendar.day:hover {
        background-color: rgba(181, 232, 224, 0.08);
      }

      calendar.other-month {
        color: rgba(205, 214, 244, 0.28);
      }

      calendar.today {
        color: #f28fad;
        font-weight: bold;
        text-decoration: underline;
      }

      calendar:selected {
        background-color: #f28fad;
        color: #1a1826;
        border-radius: 4px;
      }

      calendar button {
        background-color: transparent;
        color: #a9b1d6;
        border: none;
        box-shadow: none;
        min-width: 0;
        min-height: 0;
        padding: 2px 6px;
      }

      calendar button:hover {
        background-color: rgba(181, 232, 224, 0.08);
        color: #e8cef5;
      }

      calendar button label {
        color: inherit;
      }
    '';
  };
}
