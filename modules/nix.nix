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

    settings = {
      auto-optimise-store = true;
      experimental-features = ["nix-command" "flakes"];
      # Required by cachix
      trusted-users = ["root" "luisnquin"];
      max-jobs = 4;
    };
  };
}
