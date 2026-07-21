{
  config,
  pkgs,
  ...
}: let
  remoteHost = "rose";
  remoteHome = "/Users/${config.home.username}";
in {
  home.packages = [
    (pkgs.writeShellApplication {
      name = "simdeck";
      runtimeInputs = [pkgs.coreutils pkgs.gawk pkgs.openssh];
      text = ''
        sq=\'

        quote() {
          local s=$1
          s=''${s//$sq/$sq\\$sq$sq}
          printf '%s%s%s' "$sq" "$s" "$sq"
        }

        home=''${HOME%/}

        remote_path() {
          case $1 in
            "$home") printf '%s' '${remoteHome}' ;;
            "$home"/*) printf '%s%s' '${remoteHome}' "''${1#"$home"}" ;;
            *) printf '%s' "$1" ;;
          esac
        }

        cmd="cd $(quote "$(remote_path "$PWD")") && simdeck"

        for arg in "$@"; do
          cmd+=" $(quote "$(remote_path "$arg")")"
        done

        # Tailscale SSH never forwards the remote exit status, so carry it back as
        # a marker line and strip it on the way through. It rides stderr, not
        # stdout, so binary payloads such as `screenshot --stdout` stay byte-exact.
        status=$(mktemp)
        trap 'rm -f "$status"' EXIT

        exec 3>&1

        ssh ${remoteHost} "$cmd"'; printf "\n__simdeck_rc %s\n" $? >&2' 2>&1 1>&3 3>&- |
          awk -v f="$status" '/^__simdeck_rc /{print $2 > f; next} {print > "/dev/stderr"; fflush()}'

        exit "$(cat "$status" 2>/dev/null || echo 255)"
      '';
    })
  ];
}
