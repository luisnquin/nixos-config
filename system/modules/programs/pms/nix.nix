{
  inputs,
  system,
  pkgs,
  user,
  lib,
  ...
}: {
  nix = {
    nixPath = [
      "nixpkgs=${inputs.nixpkgs}"
    ];

    gc = {
      automatic = true;
      dates = "daily";
      options = "--delete-older-than 3d";
    };

    # Nix store
    optimise = {
      automatic = true;
      dates = ["13:00"]; # I normally lunch at this hour
    };

    # https://nixos.org/manual/nix/stable/command-ref/conf-file.html
    settings = rec {
      # Nix automatically detects files in the store that have identical contents, and replaces them with hard links to a single copy.
      auto-optimise-store = true;
      keep-outputs = true;
      warn-dirty = false;
      download-attempts = 3;
      experimental-features = ["nix-command" "flakes"];
      trusted-users = ["root" "${user.alias}"];
      allowed-users = trusted-users;
      # Defines the maximum number of jobs that Nix will try to build in parallel.
      max-jobs = 6;
      # When free disk space in /nix/store drops below min-free during a build, Nix performs a garbage-collection.
      min-free = 10000000000; # 10GB
      # Number of seconds between checking free disk space.
      min-free-check-interval = 30;
      # large builds scratch on disk instead of the tmpfs /tmp
      build-dir = "/persist/tmp/nix-builds";
      # https://nix.dev/recipes/faq#what-to-do-if-a-binary-cache-is-down-or-unreachable

      trusted-substituters = [
        "https://cache.nixos.org"
      ];
      substituters = [
        "https://cache.nixos.org"
      ];
    };
  };

  systemd.tmpfiles.rules = ["d /persist/tmp/nix-builds 0755 root root 7d"];

  programs.command-not-found = {
    enable = true;
    dbPath = lib.mkForce inputs.flake-programs-sqlite.packages.${system}.programs-sqlite;
  };

  environment = {
    systemPackages = with pkgs; [
      nix-output-monitor
      nix-prefetch-git
      cached-nix-shell
      deadnix
      statix
      nurl
      nil # lsp
      nvd # package diff
      nixgrep

      nixpkgs-review
      nixpkgs-lint

      # Formatters
      nixpkgs-fmt
      alejandra
    ];

    shellAliases = {
      nix-shell = "cached-nix-shell";
      ns = "nix-shell";
    };

    variables.NIXPKGS_ALLOW_UNFREE = "1";
  };
}
