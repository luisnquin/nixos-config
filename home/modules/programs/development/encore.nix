{
  inputs,
  system,
  ...
}: {
  programs.encore = {
    enable = true;
    settings = {
      browser = "never";
    };
  };

  home.packages = [inputs.encore.packages.${system}.encore-beta];
}
