{
  lib,
  pkgs,
  eww,
  ...
}: let
  systemInfo = pkgs.writeShellApplication {
    name = "eww-system-info";
    runtimeInputs = with pkgs; [coreutils gawk jq procps];
    text = ''
      state="/tmp/eww-system-info.state"

      read -r _ user nice system idle iowait irq softirq steal guest guest_nice < /proc/stat
      total=$((user + nice + system + idle + iowait + irq + softirq + steal + guest + guest_nice))
      idle_all=$((idle + iowait))

      prev_total=0
      prev_idle=0
      if [ -r "$state" ]; then
        read -r prev_total prev_idle < "$state" || true
      fi
      printf '%s %s\n' "$total" "$idle_all" > "$state"

      cpu_percent="$(awk -v total="$total" -v idle="$idle_all" -v ptotal="$prev_total" -v pidle="$prev_idle" '
        BEGIN {
          dt = total - ptotal;
          di = idle - pidle;
          if (dt <= 0) print 0;
          else printf "%d", ((dt - di) / dt) * 100 + 0.5;
        }')"

      cores="$(nproc)"
      load="$(cut -d' ' -f1-3 /proc/loadavg)"
      uptime_label="$(awk '{d=int($1 / 86400); h=int(($1 % 86400) / 3600); m=int(($1 % 3600) / 60); if (d > 0) printf "%dd %02dh", d, h; else printf "%dh %02dm", h, m}' /proc/uptime)"
      freq_label="$(awk -F': ' '/cpu MHz/ {sum += $2; n++} END {if (n > 0) printf "%.2f GHz", (sum / n) / 1000; else print "—"}' /proc/cpuinfo)"
      temp_label="$(awk '
        function valid(path, value) {
          getline value < path;
          close(path);
          return value != "" && value > 0;
        }
        BEGIN {
          best = "";
          for (i = 0; i < 32; i++) {
            type = "/sys/class/thermal/thermal_zone" i "/type";
            temp = "/sys/class/thermal/thermal_zone" i "/temp";
            if ((getline name < type) > 0) {
              close(type);
              if (name ~ /x86_pkg_temp|k10temp|cpu|CPU|package|Package/) {
                if (valid(temp, raw)) { best = raw; break; }
              }
            }
          }
          if (best == "") {
            for (i = 0; i < 32; i++) {
              temp = "/sys/class/thermal/thermal_zone" i "/temp";
              if (valid(temp, raw)) { best = raw; break; }
            }
          }
          if (best == "") print "—";
          else printf "%.0f°C", best / 1000;
        }')"

      mem="$(awk '
        function human(kb,   v, unit) {
          v = kb / 1024 / 1024;
          unit = "GiB";
          return sprintf("%.1f %s", v, unit);
        }
        /^MemTotal:/ {total = $2}
        /^MemAvailable:/ {avail = $2}
        /^SwapTotal:/ {swap_total = $2}
        /^SwapFree:/ {swap_free = $2}
        END {
          used = total - avail;
          pct = total > 0 ? int((used / total) * 100 + 0.5) : 0;
          swap_used = swap_total - swap_free;
          swap_pct = swap_total > 0 ? int((swap_used / swap_total) * 100 + 0.5) : 0;
          printf "%d\t%s\t%s\t%s\t%d", pct, human(used), human(total), human(swap_used), swap_pct;
        }' /proc/meminfo)"
      mem_percent="$(printf '%s' "$mem" | cut -f1)"
      mem_used="$(printf '%s' "$mem" | cut -f2)"
      mem_total="$(printf '%s' "$mem" | cut -f3)"
      swap_used="$(printf '%s' "$mem" | cut -f4)"
      swap_percent="$(printf '%s' "$mem" | cut -f5)"

      top_procs() {
        ps -eo "$1"=,comm= --sort=-"$1" \
          | awk 'NR <= 8 {v = $1; $1 = ""; sub(/^[[:space:]]+/, ""); if ($0 == "") next; printf "%s\t%.1f%%\n", $0, v}' \
          | jq -R -s -c 'split("\n") | map(select(length > 0) | split("\t") | {name: .[0], value: .[1]})'
      }
      top_cpu_all="$(top_procs %cpu)"
      top_mem_all="$(top_procs %mem)"
      top_cpu="$(printf '%s' "$top_cpu_all" | jq -c '.[0:3]')"
      top_mem="$(printf '%s' "$top_mem_all" | jq -c '.[0:3]')"

      jq -cn \
        --argjson cpu_percent "$cpu_percent" --arg cores "$cores" --arg load "$load" \
        --arg uptime "$uptime_label" --arg freq "$freq_label" --arg temp "$temp_label" \
        --argjson mem_percent "$mem_percent" --arg mem_used "$mem_used" --arg mem_total "$mem_total" \
        --arg swap_used "$swap_used" --argjson swap_percent "$swap_percent" \
        --argjson top_cpu "$top_cpu" --argjson top_cpu_all "$top_cpu_all" \
        --argjson top_mem "$top_mem" --argjson top_mem_all "$top_mem_all" \
        '{cpu_percent:$cpu_percent, cpu_label:($cpu_percent | tostring) + "%",
          cores:$cores, load:$load, uptime:$uptime, freq:$freq, temp:$temp,
          mem_percent:$mem_percent, mem_label:($mem_percent | tostring) + "%",
          mem_used:$mem_used, mem_total:$mem_total, swap_used:$swap_used,
          swap_percent:$swap_percent, swap_label:($swap_percent | tostring) + "%",
          top_cpu:$top_cpu, top_cpu_all:$top_cpu_all,
          top_mem:$top_mem, top_mem_all:$top_mem_all}'
    '';
  };
in {
  yuck = ''
    (defvar cpu_expanded false)
    (defvar mem_expanded false)

    (defpoll sys :interval "2s"
      :initial '{"cpu_percent":0,"cpu_label":"—","cores":"—","load":"—","uptime":"—","freq":"—","temp":"—","mem_percent":0,"mem_label":"—","mem_used":"—","mem_total":"—","swap_used":"—","swap_percent":0,"swap_label":"—","top_cpu":[],"top_cpu_all":[],"top_mem":[],"top_mem_all":[]}'
      `${lib.getExe systemInfo}`)

    (defwindow sysmon
      :monitor 0
      :geometry (geometry
        :x "100px"
        :y "35px"
        :width "300px"
        :anchor "top right")
      :stacking "overlay"
      :focusable false
      (sysmon-widget))

    (defwidget sysmon-widget []
      (box :class "sys-box" :orientation "v" :space-evenly false :spacing 14
        (cpu-section)
        (box :class "sys-divider")
        (mem-section)))

    (defwidget cpu-section []
      (box :class "cpu-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "sys-hero" :orientation "h" :space-evenly false :spacing 14
          (box :class "sys-meter cpu-meter" :orientation "v" :space-evenly false :valign "center"
            (label :class "sys-meter-icon" :halign "center" :text "󰍛")
            (label :class "sys-meter-value" :halign "center" :text {sys.cpu_label}))
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "sys-title" :halign "start" :text "CPU")
            (label :class "sys-subtitle" :halign "start" :text {sys.cores + " cores · " + sys.freq})
            (progress :class "sys-bar cpu-bar" :value {sys.cpu_percent})))
        (box :class "sys-grid" :orientation "v" :space-evenly false :spacing 4
          (sys-row :label "Load" :value {sys.load})
          (sys-row :label "Temp" :value {sys.temp})
          (sys-row :label "Uptime" :value {sys.uptime})
          (top-list :items {sys.top_cpu} :all {sys.top_cpu_all}
                    :expanded {cpu_expanded} :toggle "cpu_expanded"))))

    (defwidget mem-section []
      (box :class "mem-box" :orientation "v" :space-evenly false :spacing 12
        (box :class "sys-hero" :orientation "h" :space-evenly false :spacing 14
          (box :class "sys-meter mem-meter" :orientation "v" :space-evenly false :valign "center"
            (label :class "sys-meter-icon" :halign "center" :text "")
            (label :class "sys-meter-value" :halign "center" :text {sys.mem_label}))
          (box :orientation "v" :space-evenly false :hexpand true :halign "start"
            (label :class "sys-title" :halign "start" :text "Memory")
            (label :class "sys-subtitle" :halign "start" :text {sys.mem_used + " / " + sys.mem_total})
            (progress :class "sys-bar mem-bar" :value {sys.mem_percent})))
        (box :class "sys-grid" :orientation "v" :space-evenly false :spacing 4
          (sys-row :label "Used" :value {sys.mem_used})
          (sys-row :label "Total" :value {sys.mem_total})
          (sys-row :label "Swap" :value {sys.swap_used + " · " + sys.swap_label})
          (top-list :items {sys.top_mem} :all {sys.top_mem_all}
                    :expanded {mem_expanded} :toggle "mem_expanded"))))

    (defwidget sys-row [label value]
      (box :class "sys-row" :orientation "h" :space-evenly false
        (label :class "sys-row-label" :halign "start" :hexpand true :text label)
        (label :class "sys-row-value" :halign "end" :text value)))

    (defwidget top-list [items all expanded toggle]
      (box :class "top-list" :orientation "v" :space-evenly false :spacing 4
        (box :class "top-head" :orientation "h" :space-evenly false
          (label :class "top-head-label" :halign "start" :hexpand true :text "Top processes")
          (button :class "top-toggle"
            :onclick {"${eww} update " + toggle + "=" + (expanded ? "false" : "true")}
            (label :text {expanded ? "− less" : "+ more"})))
        (for p in {expanded ? all : items}
          (proc-row :name {p.name} :value {p.value}))))

    (defwidget proc-row [name value]
      (box :class "sys-row proc-row" :orientation "h" :space-evenly false
        (label :class "sys-row-label proc-name" :halign "start" :hexpand true :limit-width 24 :text name)
        (label :class "sys-row-value proc-value" :halign "end" :text value)))
  '';
}
