{
  lib,
  pkgs,
  eww,
  ...
}: let
  tailscaleInfo = pkgs.writeShellApplication {
    name = "eww-tailscale-info";
    runtimeInputs = with pkgs; [coreutils jq tailscale];
    text = ''
      if ! json="$(tailscale status --json 2>/dev/null)"; then
        jq -cn '{connected:false,online_count:0,total_count:0,self_name:"—",self_ip:"—",tailnet:"—",devices:[]}'
        exit 0
      fi

      printf '%s' "$json" | jq -c '
        now as $now
        | def osmeta:
            {
              "linux":   {icon:"\uf17c", label:"Linux"},
              "macOS":   {icon:"\uf179", label:"macOS"},
              "iOS":     {icon:"\uf10b", label:"iOS"},
              "android": {icon:"\uf17b", label:"Android"},
              "windows": {icon:"\uf17a", label:"Windows"},
              "freebsd": {icon:"\uf30c", label:"FreeBSD"}
            }[.] // {icon:"\uf108", label:(if (. == null or . == "") then "unknown" else . end)};
          def rel($d):
            if $d < 60 then "just now"
            elif $d < 3600 then (($d/60|floor|tostring) + "m ago")
            elif $d < 86400 then (($d/3600|floor|tostring) + "h ago")
            elif $d < 2592000 then (($d/86400|floor|tostring) + "d ago")
            else (($d/2592000|floor|tostring) + "mo ago") end;
          def mk($isSelf):
            (.OS // "" | osmeta) as $os
            | (.Online // false) as $on
            | (.LastSeen // "" | sub("\\.[0-9]+";"") | try fromdateiso8601 catch 0) as $ls
            | {
                name: (.HostName // "device"),
                os: $os.label,
                os_icon: $os.icon,
                online: $on,
                is_self: $isSelf,
                ip: (.TailscaleIPs[0] // "—"),
                status: (if $isSelf then "this device"
                         elif $on then "online"
                         elif $ls > 0 then rel($now - $ls)
                         else "unknown" end),
                status_class: (if $on then "online" else "offline" end),
                sort_key: (if $on then $now else $ls end)
              };
          (.Self // {}) as $self
          | ([ ($self | mk(true)) ] + [ (.Peer // {} | to_entries[] | .value | mk(false)) ]) as $all
          | ($all | sort_by(.sort_key) | reverse) as $devices
          | {
              connected: true,
              online_count: ($devices | map(select(.online)) | length),
              total_count: ($devices | length),
              self_name: ($self.HostName // "—"),
              self_ip: ($self.TailscaleIPs[0] // "—"),
              tailnet: ($self.DNSName // "" | rtrimstr(".")
                        | (split(".") | if length > 1 then (.[1:] | join(".")) else "tailnet" end)),
              devices: $devices
            }'
    '';
  };
in {
  yuck = ''
    (defpoll ts :interval "5s"
      :initial '{"connected":false,"online_count":0,"total_count":0,"self_name":"—","self_ip":"—","tailnet":"—","devices":[]}'
      `${lib.getExe tailscaleInfo}`)

    (defwindow tailscale
      :monitor 0
      :geometry (geometry
        :x "160px"
        :y "35px"
        :width "320px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (ts-widget))

    (defwidget ts-widget []
      (box :class "ts-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "ts-header" :orientation "h" :space-evenly false :spacing 10
          (label :class {"ts-mark " + (ts.connected ? "up" : "down")} :text "")
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "ts-title" :halign "start" :text "Tailscale")
            (label :class "ts-meta" :halign "start"
              :text {ts.connected
                ? (ts.online_count + "/" + ts.total_count + " online · " + ts.tailnet)
                : "disconnected"}))
          (label :class "ts-self-ip" :valign "start" :text {ts.self_ip}))
        (scroll :class "ts-scroll" :vscroll true :height 280
          (box :class "ts-list" :orientation "v" :space-evenly false :spacing 5
            (for d in {ts.devices}
              (box :class {"ts-entry " + d.status_class + (d.is_self ? " self" : "")}
                   :orientation "h" :space-evenly false :spacing 10
                (label :class {"ts-os " + d.status_class} :text {d.os_icon})
                (box :orientation "v" :space-evenly false :hexpand true :halign "start"
                  (label :class "ts-name" :halign "start" :limit-width 24 :text {d.name})
                  (label :class "ts-sub" :halign "start" :text {d.os + " · " + d.ip}))
                (box :orientation "v" :space-evenly false :halign "end"
                  (label :class {"ts-dot " + d.status_class} :halign "end" :text {d.online ? "●" : "○"})
                  (label :class "ts-time" :halign "end" :text {d.status}))))))
        (box :class "ts-actions" :orientation "h" :space-evenly true :spacing 8
          (button :class "ts-btn" :onclick "${eww} poll ts" "refresh")
          (button :class "ts-btn"
            :onclick {"${eww} close tailscale & ${lib.getExe' pkgs.xdg-utils "xdg-open"} https://login.tailscale.com/admin/machines"}
            "admin panel"))))
  '';
}
