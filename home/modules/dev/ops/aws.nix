{
  config,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      aws-lambda-rie
      pkgs.extra.s3-edit
      pkgs.extra.ecsview
      (awscli2.overrideAttrs
        (_old: {
          disabledTestPaths = [
            "tests/dependencies"
            "tests/unit/botocore"
            "tests/unit/customizations/logs"
            "tests/unit/customizations/cloudtrail"

            # Integration tests require networking
            "tests/integration"

            # Disable slow tests (only run unit tests)
            "tests/backends"
            "tests/functional"
          ];
        }))
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
