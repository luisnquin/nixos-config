{pkgs, ...}: let
  phone = pkgs.writeShellApplication {
    name = "phone";
    runtimeInputs = with pkgs; [
      android-tools
      coreutils
      fzf
      jq
      openssh
      scrcpy
      wl-clipboard
    ];
    text = builtins.readFile ./phone.sh;
  };

  completions = pkgs.writeTextFile {
    name = "phone-zsh-completions";
    destination = "/share/zsh/site-functions/_phone";
    text = builtins.readFile ./_phone.zsh;
  };
in {
  home.packages = [
    (pkgs.symlinkJoin {
      name = "phone-with-completions";
      paths = [phone completions];
    })
  ];
}
