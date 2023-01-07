{
  nix = {
    settings = {
      substituters = [
        "https://nix-linter.cachix.org"
      ];
      trusted-public-keys = [
        "nix-linter.cachix.org-1:BdTne5LEHQfIoJh4RsoVdgvqfObpyHO5L0SCjXFShlE="
      ];
    };
  };
}
