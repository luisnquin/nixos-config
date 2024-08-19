{
  config,
  pkgsx,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      aws-lambda-rie
      pkgsx.s3-edit
      pkgsx.ecsview
      awscli2
      stu
    ];

    sessionVariables = rec {
      # default profiles are very ambiguous
      AWS_DEFAULT_PROFILE = "personal";
      AWS_PROFILE = AWS_DEFAULT_PROFILE;
      AWS_REGION = "sa-east-1";
    };

    file = let
      inherit (config.home) homeDirectory;
    in {
      ".stu/config.toml".text = ''
        download_dir = "${homeDirectory}/Downloads/s3"
      '';
    };
  };
}
