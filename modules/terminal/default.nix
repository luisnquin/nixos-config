{
  config,
  pkgs,
  ...
}: {
  imports = [
    "/etc/nixos/modules/terminal/alacritty.nix"
    "/etc/nixos/modules/terminal/starship.nix"
    "/etc/nixos/modules/terminal/tmux.nix"
    "/etc/nixos/modules/terminal/zsh.nix"
  ];

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  environment = {
    # Google search, zsh history search, highlighter for conventional
    # branches and tmux startup in non-vscode editors
    #
    # Shell script code called during interactive shell initialisation.
    # This code is assumed to be shell-independent, which means you
    # should stick to pure sh without sh word split.

    interactiveShellInit = builtins.readFile ../../dots/.shrc;

    systemPackages = with pkgs; [
      cached-nix-shell
    ];

    variables = {
      # The other related config only apply to the build
      NIXPKGS_ALLOW_UNFREE = "1";
      PATH = "$PATH:$GORROT:$GOPATH/bin";
      GOPATH = "/home/$USER/go";
      EDITOR = "nano";
    };

    shellAliases = {
      gest = "go clean -testcache && richgo test -v";
      setup = "tmux rename-window \"setup 🦭\" \\; split-window -h \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; split-window -v \\; resize-pane -D 3 \\; selectp -t 0 \\; send-keys -t 1 \"gotop --nvidia --color=vice\" ENTER \\; send-keys -t 3 \"k9s  --readonly\" ENTER; clear";

      # It's not like you're not needed
      runds = "(ds; rm -rf compose/nginx/env.json && make compose-up && make build && make run)";
      v3 = "cd ~/go/src/gitlab.wiserskills.net/wiserskills/v3/";
      ds = "v3 && cd dataserver/";

      # Instant tp to some directories
      dot = "cd ~/.dotfiles/";
      down = "cd ~/Downloads/";
      etc = "cd ~/.etc/";
      gopl = "cd ~/Workspace/playground/go/";
      rustpl = "cd ~/Workspace/playground/rust/";
      pl = "cd ~/Workspace/playground/";
      pr = "cd ~/Workspace/projects/";
      tests = "cd ~/Tests/";
      tmp = "cd /tmp/";
      workspace = "cd ~/Workspace/";

      # Overwriten program calls
      rm = "rm --interactive";
      du = "du --human-readable";
      xclip = "xclip -selection c";
      ls = "exa --icons";
      ll = "exa -l";
      la = "exa -a";
      cat = "bat -p";

      ".." = "cd ..";
      "..." = "cd ../..";

      nix-shell = "cached-nix-shell";
      ns = "nix-shell";

      # Those who are lazy to write definitely go here
      "~" = "cd /home/$USER/";
      open = "xdg-open";
      rc = "rclone";
      cls = "clear";
      tt = "ranger";
      poff = "poweroff";
      actl = "act --list";
      neofetch = "freshfetch";
      # The most useful alias, lol
      gotry = "xdg-open https://go.dev/play >>/dev/null";
      whoseport = "netstat -tulpln 2> /dev/null | grep :";
      nyancat = "nyancat --no-counter";
      ale = "alejandra --quiet";
      dud = "du --human-readable --summarize";
      man = "tldr";
      transg = "transgression-tui";
      # Abstraction
      listen = "ngrok http";
      py = "python3";
      share = "ngrok http";
      top = "gotop --nvidia --color=vice";
      unrar = "unar";
    };

    shells = [pkgs.zsh];
  };
}
