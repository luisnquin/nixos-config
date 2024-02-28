{
  pkgsx,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      aws-lambda-rie
      pkgsx.ecsview
      pkgsx.stu
      awscli2
    ];

    sessionVariables = rec {
      # default profiles are very ambiguous
      AWS_DEFAULT_PROFILE = "personal";
      AWS_PROFILE = AWS_DEFAULT_PROFILE;
    };
  };
}
