{
  pkgs-extra,
  config,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      aws-lambda-rie
      pkgs-extra.s3-edit
      pkgs-extra.ecsview
      (awscli2.overrideAttrs
        (_old: {
          doCheck = false;
        }))
      stu
    ];

    sessionVariables = rec {
      AWS_DEFAULT_PROFILE = "default";
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
