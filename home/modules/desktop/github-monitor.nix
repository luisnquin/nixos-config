{
  services.github-monitor = {
    enable = true;
    workflows = [
      {
        repo = "luisnquin/nixos-config";
        workflow = "Check Nix formatting";
      }
      {
        repo = "cuentacero/sevastopol";
        workflow = "App Quality";
      }
      {
        repo = "cuentacero/gate-k9";
        workflow = "check";
      }
    ];
    issueRepositories = [
      "luisnquin/nixos-config"
      "cuentacero/sevastopol"
      "cuentacero/gate-k9"
      "0xc000022070/zen-browser-flake"
    ];
  };
}
