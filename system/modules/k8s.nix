{pkgs, ...}: {
  environment = {
    systemPackages = with pkgs; [
      kubernetes
      minikube
      kubectl
      # lens
      k9s
    ];

    shellAliases = {
      # The main config.yml is generated "in runtime" and there's no other
      # way without overdo with `secrets.nix`
      k9s = "k9s --crumbsless --logoless --refresh=2";
      kw = "k9s";
      k = "kubectl";
    };
  };
}
