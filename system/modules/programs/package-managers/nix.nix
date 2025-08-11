{
  inputs,
  pkgs,
  user,
  ...
}: {
  nix = {
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
      # Required by cachix
      trusted-users = ["root" "${user.alias}"];
      allowed-users = trusted-users;
      # Defines the maximum number of jobs that Nix will try to build in parallel.
      max-jobs = 6;
      # When free disk space in /nix/store drops below min-free during a build, Nix performs a garbage-collection.
      min-free = 10000000000; # 10GB
      # Number of seconds between checking free disk space.
      min-free-check-interval = 30;
      # https://nix.dev/recipes/faq#what-to-do-if-a-binary-cache-is-down-or-unreachable
      trusted-public-keys = [
        "nix-gaming.cachix.org-1:nbjlureqMbRAxR1gJ/f3hxemL9svXaZF/Ees8vCUUs4="
      ];
      trusted-substituters = [
        "https://cache.nixos.org"
      ];
      substituters = [
        "https://nix-gaming.cachix.org"
        "https://cache.nixos.org"
      ];
    };
  };

  tools.nyx.enable = true;

  programs.command-not-found.enable = true;

  environment = {
    systemPackages = with pkgs; let
      inherit (inputs.nix-scripts.packages.${system}) nixgrep;
    in [
      nix-output-monitor
      nix-prefetch-git
      cached-nix-shell
      deadnix
      nixgrep
      statix
      nurl
      nil # lsp
      nvd # package diff

      nixpkgs-review
      nixpkgs-lint

      # Formatters
      nixfmt-classic
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
