{
  pkgsx,
  pkgs,
  ...
}: {
  home.packages = with pkgs; [
    aws-lambda-rie
    pkgsx.stu
    awscli2
  ];
}
