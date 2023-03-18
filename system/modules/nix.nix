{config, ...}: {
  nixpkgs.config = {
    permittedInsecurePackages = [
      "electron-12.2.3"
    ];
    allowBroken = false;
    # The day I meet the man who has this option in false
    allowUnfree = true;
  };

  nix = {
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-old";
    };

    # Nix store
    optimise = {
      automatic = true;
      dates = ["13:00"];
    };

    # Ref: https://nixos.org/manual/nix/stable/command-ref/conf-file.html
    settings = {
      # Nix automatically detects files in the store that have identical contents, and replaces them with hard links to a single copy.
      auto-optimise-store = true;
      experimental-features = ["nix-command" "flakes"];
      keep-outputs = true;
      download-attempts = 3;
      # Required by cachix
      trusted-users = ["root" "luisnquin"];
      # Defines the maximum number of jobs that Nix will try to build in parallel.
      max-jobs = 4;
      # When free disk space in /nix/store drops below min-free during a build, Nix performs a garbage-collection.
      min-free = 10000000000; # 10GB
      # Number of seconds between checking free disk space.
      min-free-check-interval = 30;
    };
  };
}