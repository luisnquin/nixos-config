{
  config,
  lib,
  pkgs,
  eww,
  ...
}: let
  nmtuiCmd = "${lib.getExe config.programs.ghostty.package} --class=eww.nmtui -e ${pkgs.networkmanager}/bin/nmtui connect";

  scanCmd = "${lib.getExe pkgs.scripts.nmcli-wifi-scan-waybar} --scan";

  netInfo = pkgs.writeShellApplication {
    name = "eww-network-info";
    runtimeInputs = with pkgs; [iproute2 iw coreutils gawk jq];
    text = ''
      state="/tmp/eww-network-rate.state"

      dev="$(ip route show default 2>/dev/null | awk '/^default/ {print $5; exit}')"

      if [ -z "$dev" ]; then
        jq -cn '{connected:false, icon:"󰤮", primary:"Disconnected", subtitle:"no route", signal_label:"", ip:"—", gateway:"—", freq_label:"—", up:"0 B/s", down:"0 B/s"}'
        exit 0
      fi

      rx="$(cat "/sys/class/net/$dev/statistics/rx_bytes" 2>/dev/null || echo 0)"
      tx="$(cat "/sys/class/net/$dev/statistics/tx_bytes" 2>/dev/null || echo 0)"
      now="$(date +%s%N)"

      p_dev=""; p_rx=0; p_tx=0; p_now=0
      if [ -r "$state" ]; then
        read -r p_dev p_rx p_tx p_now < "$state" || true
      fi
      printf '%s %s %s %s\n' "$dev" "$rx" "$tx" "$now" > "$state"

      rates="$(awk -v dev="$dev" -v rx="$rx" -v tx="$tx" -v now="$now" \
                   -v pdev="$p_dev" -v prx="$p_rx" -v ptx="$p_tx" -v pnow="$p_now" '
        function human(b,   u, i) {
          split("B/s kB/s MB/s GB/s", u, " ");
          i = 1; while (b >= 1024 && i < 4) { b /= 1024; i++ }
          return sprintf("%.1f %s", b, u[i]);
        }
        BEGIN {
          if (pdev != dev || pnow == 0 || now <= pnow) { up = 0; down = 0 }
          else {
            dt = (now - pnow) / 1e9;
            up = (tx - ptx) / dt; down = (rx - prx) / dt;
            if (up < 0) up = 0; if (down < 0) down = 0;
          }
          printf "%s\t%s", human(up), human(down);
        }')"
      up="$(printf '%s' "$rates" | cut -f1)"
      down="$(printf '%s' "$rates" | cut -f2)"

      ip4="$(ip -4 -o addr show dev "$dev" 2>/dev/null | awk '{print $4}' | cut -d/ -f1 | head -n1)"
      [ -z "$ip4" ] && ip4="—"
      gw="$(ip -4 route show default dev "$dev" 2>/dev/null | awk '/default/ {print $3; exit}')"
      [ -z "$gw" ] && gw="—"

      if [ -d "/sys/class/net/$dev/wireless" ]; then
        link="$(iw dev "$dev" link 2>/dev/null || true)"
        ssid="$(printf '%s\n' "$link" | awk -F': ' '/SSID/ {print $2; exit}')"
        [ -z "$ssid" ] && ssid="$dev"
        dbm="$(printf '%s\n' "$link" | awk '/signal:/ {print $2; exit}')"
        freq="$(printf '%s\n' "$link" | awk '/freq:/ {printf "%d", $2; exit}')"
        if [ -n "$freq" ]; then freq_label="$freq MHz"; else freq_label="—"; fi
        if [ -n "$dbm" ]; then
          signal_label="$dbm dBm"
          pct="$(awk -v d="$dbm" 'BEGIN { p = 2 * (d + 100); if (p < 0) p = 0; if (p > 100) p = 100; print int(p) }')"
        else
          signal_label=""; pct=0
        fi
        icon="$(awk -v p="$pct" 'BEGIN {
          if (p >= 80) print "󰤨"; else if (p >= 60) print "󰤥";
          else if (p >= 40) print "󰤢"; else if (p >= 20) print "󰤟"; else print "󰤯" }')"
        primary="$ssid"
        subtitle="$dev · Wi-Fi"
      else
        icon="󰈀"
        primary="Ethernet"
        subtitle="$dev · wired"
        signal_label=""
        freq_label="—"
      fi

      jq -cn \
        --arg icon "$icon" --arg primary "$primary" --arg subtitle "$subtitle" \
        --arg signal_label "$signal_label" --arg ip "$ip4" --arg gateway "$gw" \
        --arg freq_label "$freq_label" --arg up "$up" --arg down "$down" \
        '{connected:true, icon:$icon, primary:$primary, subtitle:$subtitle,
          signal_label:$signal_label, ip:$ip, gateway:$gateway,
          freq_label:$freq_label, up:$up, down:$down}'
    '';
  };
in {
  yuck = ''
    (defpoll net :interval "2s"
      :initial '{"connected":false,"icon":"󰤫","primary":"…","subtitle":"","signal_label":"","ip":"—","gateway":"—","freq_label":"—","up":"0 B/s","down":"0 B/s"}'
      `${lib.getExe netInfo}`)

    (defwindow network
      :monitor 0
      :geometry (geometry
        :x "50px"
        :y "35px"
        :width "260px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (net-widget))

    (defwidget net-widget []
      (box :class "net-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "net-header" :orientation "h" :space-evenly false :spacing 10
          (label :class "net-icon" :text {net.icon})
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "net-ssid" :halign "start" :limit-width 18 :text {net.primary})
            (label :class "net-type" :halign "start" :text {net.subtitle}))
          (label :class "net-dbm" :valign "start" :text {net.signal_label}))
        (box :orientation "v" :space-evenly false :spacing 4
          (net-row :label "IP" :value {net.ip})
          (net-row :label "Gateway" :value {net.gateway})
          (net-row :label "Frequency" :value {net.freq_label}))
        (box :class "net-rates" :orientation "h" :space-evenly true
          (label :class "net-up" :text {"↑ " + net.up})
          (label :class "net-down" :text {"↓ " + net.down}))
        (box :class "net-actions" :orientation "h" :space-evenly true :spacing 8
          (button :class "net-btn" :onclick "${eww} close network & ${nmtuiCmd}" "nmtui")
          (button :class "net-btn" :onclick "${scanCmd}" "force scan"))))

    (defwidget net-row [label value]
      (box :class "net-row" :orientation "h" :space-evenly false
        (label :class "net-row-label" :halign "start" :hexpand true :text label)
        (label :class "net-row-value" :halign "end" :text value)))
  '';
}
