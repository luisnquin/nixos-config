{pkgs, ...}: {
  home.packages = with pkgs; [
    aws-lambda-rie
    awscli
  ];
}
