# It includes Node.js runtimes and package managers
{
  config,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      nodePackages.pnpm
      nodejs_21
      bun
    ];

    file.".npmrc".text = ''
      prefix=${config.home.homeDirectory}/.npm-global
    '';
  };
}
