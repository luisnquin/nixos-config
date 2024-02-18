#!/bin/sh

netscan() {
    nmcli radio wifi on && nmcli --fields SSID,SECURITY,BARS device wifi list ifname "$(nmcli device | awk '$2 == "wifi" {print $1}')" --rescan yes
}
