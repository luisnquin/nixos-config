{
  inputs,
  pkgs,
  ...
}: {
  home.packages = [
    inputs.encore.packages.${pkgs.system}.encore
  ];

  xdg.configFile = {
    "encore/config".source = (pkgs.formats.toml {}).generate "encore-config" {
      run = {
        browser = "never";
      };
    };
  };
}
