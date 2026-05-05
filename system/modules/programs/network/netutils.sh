#!/bin/sh

netscan() {
    nmcli radio wifi on && nmcli --fields SSID,SECURITY,BARS device wifi list ifname "$(nmcli device | awk '$2 == "wifi" {print $1}')" --rescan yes
}

netport() {
    sudo ss -lptn "sport = :$1"
}

myip() {
    curl -s ifconfig.me && echo
}

wake_host() {
    host="${1:?usage: wake_host <host> [broadcast]}"
    broadcast="${2:-255.255.255.255}"

    ip="$(
        getent ahostsv4 "$host" 2>/dev/null |
            awk 'NR == 1 { print $1 }'
    )"

    if [ -z "$ip" ]; then
        echo "could not resolve host: $host" >&2
        return 1
    fi

    mac="$(
        ip neigh show "$ip" 2>/dev/null |
            awk '
        {
          for (i = 1; i <= NF; i++) {
            if ($i == "lladdr") {
              print $(i + 1)
              exit
            }
          }
        }
      '
    )"

    if [ -z "$mac" ]; then
        ping -c1 -W1 "$ip" >/dev/null 2>&1 || true

        mac="$(
            ip neigh show "$ip" 2>/dev/null |
                awk '
          {
            for (i = 1; i <= NF; i++) {
              if ($i == "lladdr") {
                print $(i + 1)
                exit
              }
            }
          }
        '
        )"
    fi

    if [ -z "$mac" ]; then
        echo "could not discover MAC for $host ($ip)" >&2
        return 1
    fi

    wakeonlan -i "$broadcast" "$mac"
}
