{pkgs, ...}: {
  nix = {
    gc = {
      automatic = true;
      dates = "daily";
      options = "--delete-older-than 3d";
    };

    # Nix store
    optimise = {
      automatic = true;
      dates = ["13:00"];
    };

    # Ref: https://nixos.org/manual/nix/stable/command-ref/conf-file.html
    extraOptions = builtins.readFile ../dots/nix.conf;
    settings = {
      # Nix automatically detects files in the store that have identical contents, and replaces them with hard links to a single copy.
      auto-optimise-store = true;
      keep-outputs = true;
      warn-dirty = false;
      download-attempts = 3;
      # Required by cachix
      trusted-users = ["root" "luisnquin"];
      # Defines the maximum number of jobs that Nix will try to build in parallel.
      max-jobs = 6;
      # When free disk space in /nix/store drops below min-free during a build, Nix performs a garbage-collection.
      min-free = 10000000000; # 10GB
      # Number of seconds between checking free disk space.
      min-free-check-interval = 30;
      # https://nix.dev/recipes/faq#what-to-do-if-a-binary-cache-is-down-or-unreachable
      trusted-substituters = ["https://cache.nixos.org"];
      substituters = ["https://cache.nixos.org"];
    };
  };

  nixpkgs.config = {
    permittedInsecurePackages = [
      "electron-12.2.3"
      "nodejs-16.20.2"
    ];
    allowBroken = false;
    allowUnfree = true;
  };

  environment = {
    systemPackages = with pkgs; [
      nix-output-monitor
      nix-prefetch-git
      cached-nix-shell
      alejandra
      rnix-lsp
      deadnix
      statix
      nurl
    ];

    shellAliases = {
      nix-shell = "cached-nix-shell";
      ns = "nix-shell";
    };

    variables.NIXPKGS_ALLOW_UNFREE = "1";
  };
}
