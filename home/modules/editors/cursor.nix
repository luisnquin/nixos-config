{pkgs, ...}: {
  home.packages = [
    pkgs.re.code-cursor
  ];

  # so the process is detached from the terminal
  programs.zsh.initContent = ''
    cursor() {
      (nohup cursor "$@" >/dev/null 2>&1 &)
    }
  '';
}
