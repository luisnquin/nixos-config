{
  name,
  eww,
  lib,
  pkgs,
  ...
}: let
  grant = pkgs.writeShellApplication {
    name = "eww-phone-grant";
    runtimeInputs = [pkgs.phone pkgs.libnotify];
    text = ''
      if ! out=$(phone grant "$@" 2>&1); then
        notify-send --app-name=phone-grant "phone grant failed" "$out"
      fi
      ${eww} update phone_grants="$(phone grant --json)"
    '';
  };
  act = verb: args: label: show: ''
    (button :class "phone-act ${verb}" :visible {${show}}
      :onclick "${lib.getExe grant} ''${d.name} ${args}" (label :text "${label}"))'';
in {
  yuck = ''
    (defpoll phone_grants :interval "5s"
      :initial '{"devices":[]}'
      `${lib.getExe pkgs.phone} grant --json`)

    (defwindow ${name}
      :monitor 0
      :geometry (geometry
        :x "180px"
        :y "35px"
        :width "320px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (phone-widget))

    (defwidget phone-widget []
      (box :class "phone-box" :orientation "v" :space-evenly false :spacing 2
        (label :class "phone-empty" :halign "start" :visible {arraylength(phone_grants.devices) == 0}
          :text "no handsets seen this week")
        (for d in {phone_grants.devices}
          (box :class {"phone-device " + d.state} :orientation "h" :space-evenly false :spacing 4
            (box :orientation "v" :space-evenly false :hexpand true
              (label :class "phone-name" :halign "start" :text {d.name})
              (label :class "phone-detail" :halign "start" :limit-width 34 :text {d.detail}))
            ${act "allow" "allow" "allow" ''d.state != "held" && d.state != "granted"''}
            ${act "window" "allow --for 15m" "15m" ''d.state != "held" && d.state != "granted"''}
            ${act "refuse" "refuse" "refuse" ''d.state == "asking"''}
            ${act "stop" "stop" "stop" ''d.state == "held" || d.state == "granted"''}))))
  '';
}
