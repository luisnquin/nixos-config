{
  config,
  lib,
  ...
}: let
  inherit (lib) mkOption types mkIf;

  cfg = config.programs.claude-code;

  # always allowed regardless of profile
  baseAllow = [
    "Glob(*)"
    "Grep(*)"
    "LS(*)"
    "Read(*)"
    "Search(*)"
    "Task(*)"
    "TodoWrite(*)"

    "Bash(git status)"
    "Bash(git log:*)"
    "Bash(git diff:*)"
    "Bash(git show:*)"
    "Bash(git branch:*)"
    "Bash(git remote:*)"

    "Bash(ls:*)"
    "Bash(find:*)"
    "Bash(cat:*)"
    "Bash(head:*)"
    "Bash(tail:*)"

    "Bash(nix eval:*)"
    "Bash(nix flake show:*)"
    "Bash(nix flake metadata:*)"

    # "mcp__github__search_repositories"
    # "mcp__github__get_file_contents"
    # "mcp__sequential-thinking__sequentialthinking"

    # "mcp__filesystem__read_file"
    # "mcp__filesystem__read_text_file"
    # "mcp__filesystem__read_media_file"
    # "mcp__filesystem__read_multiple_files"
    # "mcp__filesystem__list_directory"
    # "mcp__filesystem__list_directory_with_sizes"
    # "mcp__filesystem__directory_tree"
    # "mcp__filesystem__search_files"
    # "mcp__filesystem__get_file_info"
    # "mcp__filesystem__list_allowed_directories"

    "WebFetch(domain:github.com)"
    "WebFetch(domain:wiki.hyprland.org)"
    "WebFetch(domain:wiki.hypr.land)"
    "WebFetch(domain:raw.githubusercontent.com)"
    "WebFetch(domain:encore.dev)"
  ];

  # Standard profile additions - balanced permissions
  standardAllow =
    baseAllow
    ++ [
      "Bash(git add:*)"

      "Bash(nix:*)"

      "Bash(mkdir:*)"
      "Bash(chmod:*)"

      "Bash(rg:*)"
      "Bash(grep:*)"

      "Bash(systemctl list-units:*)"
      "Bash(systemctl list-timers:*)"
      "Bash(systemctl status:*)"
      "Bash(journalctl:*)"
      "Bash(dmesg:*)"
      "Bash(env)"
      "Bash(claude --version)"
      "Bash(nh search:*)"

      "Bash(pactl list:*)"
      "Bash(pw-top)"

      "Bash(hyprctl dispatch:*)"

      "Bash(coredumpctl list:*)"

      "Bash(encore:*)"
      "Bash(bun dev)"
      "Bash(npx prisma generate)"
      "Bash(npx prisma validate)"
      "Bash(npx prisma format)"
      "Bash(npx prisma migrate dev)"
      "Bash(npx prisma migrate status)"

      # Additional home directory reads
      # "Read(${config.home.homeDirectory}/Documents/github/home-manager/**)"
      # "Read(${config.home.homeDirectory}/.config/sway/**)"
    ];

  # full autonomy for trusted workflows
  autonomousAllow =
    standardAllow
    ++ [
      "Bash(git commit:*)"
      "Bash(git checkout:*)"
      "Bash(git switch:*)"
      "Bash(git stash:*)"
      "Bash(git restore:*)"
      "Bash(git reset:*)"

      "Bash(rm:*)"
    ];

  # Operations requiring confirmation in non-autonomous mode
  standardAsk = [
    "Bash(git checkout:*)"
    "Bash(git commit:*)"
    "Bash(git merge:*)"
    "Bash(git pull:*)"
    "Bash(git push:*)"
    "Bash(git rebase:*)"
    "Bash(git reset:*)"
    "Bash(git restore:*)"
    "Bash(git stash:*)"
    "Bash(git switch:*)"

    "Bash(cp:*)"
    "Bash(mv:*)"
    "Bash(rm:*)"

    "Bash(systemctl disable:*)"
    "Bash(systemctl enable:*)"
    "Bash(systemctl mask:*)"
    "Bash(systemctl reload:*)"
    "Bash(systemctl restart:*)"
    "Bash(systemctl start:*)"
    "Bash(systemctl stop:*)"
    "Bash(systemctl unmask:*)"

    "Bash(curl:*)"
    "Bash(ping:*)"
    "Bash(rsync:*)"
    "Bash(scp:*)"
    "Bash(ssh:*)"
    "Bash(wget:*)"

    "Bash(nixos-rebuild:*)"
    "Bash(sudo:*)"

    "Bash(npx prisma db push)"
    "Bash(npx prisma migrate deploy)"

    "Bash(kill:*)"
    "Bash(killall:*)"
    "Bash(pkill:*)"
  ];

  # Autonomous mode still requires confirmation for these
  autonomousAsk = [
    "Bash(git push:*)"
    "Bash(git merge:*)"
    "Bash(git rebase:*)"

    "Bash(systemctl:*)"
    "Bash(nixos-rebuild:*)"
    "Bash(sudo:*)"

    "Bash(curl:*)"
    "Bash(rsync:*)"
    "Bash(scp:*)"
    "Bash(ssh:*)"
    "Bash(wget:*)"

    "Bash(kill:*)"
    "Bash(killall:*)"
    "Bash(pkill:*)"
  ];

  # Never allowed
  denyList = [
    "Bash(rm -rf /*)"
    "Bash(rm -rf /)"
    "Bash(dd:*)"
    "Bash(mkfs:*)"

    "Bash(npx prisma migrate reset)"
    "Bash(npx prisma studio)"
    "Bash(npx prisma dev)"

    "Bash(encore secret set:*)"
    "Bash(encore secret delete:*)"
  ];
in {
  options.programs.claude-code.permissionProfile = mkOption {
    type = types.enum [
      "conservative"
      "standard"
      "autonomous"
    ];
    default = "standard";
    description = ''
      Permission profile for Claude Code operations:
      - conservative: Minimal permissions, most operations require confirmation
      - standard: Balanced permissions for normal development workflows
      - autonomous: Maximum autonomy for trusted environments
    '';
  };

  config = mkIf cfg.enable {
    programs.claude-code.settings.permissions = {
      allow =
        if cfg.permissionProfile == "autonomous"
        then autonomousAllow
        else if cfg.permissionProfile == "standard"
        then standardAllow
        else baseAllow;

      ask =
        if cfg.permissionProfile == "autonomous"
        then autonomousAsk
        else if cfg.permissionProfile == "standard"
        then standardAsk
        else standardAsk ++ standardAllow;

      deny = denyList;

      defaultMode =
        if cfg.permissionProfile == "autonomous"
        then "acceptEdits"
        else "default";
    };
  };
}
