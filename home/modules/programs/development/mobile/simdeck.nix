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
      runtimeInputs = [pkgs.openssh];
      text = ''
        sq=\'

        quote() {
          local s=$1
          s=''${s//$sq/$sq\\$sq$sq}
          printf '%s%s%s' "$sq" "$s" "$sq"
        }

        remote_path() {
          case $1 in
            "$HOME") printf '%s' '${remoteHome}' ;;
            "$HOME"/*) printf '%s%s' '${remoteHome}' "''${1#"$HOME"}" ;;
            *) printf '%s' "$1" ;;
          esac
        }

        cmd="cd $(quote "$(remote_path "$PWD")") && simdeck"

        for arg in "$@"; do
          cmd+=" $(quote "$(remote_path "$arg")")"
        done

        exec ssh ${remoteHost} "$cmd"
      '';
    })
  ];
}
