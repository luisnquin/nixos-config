{pkgs, ...}: {
  home.packages = with pkgs; [
    kubernetes
    minikube
    kubectl
  ];
}
