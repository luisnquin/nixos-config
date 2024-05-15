# It includes Node.js runtimes and package managers
{
  config,
  pkgs,
  ...
}: {
  home = {
    packages = with pkgs; [
      nodePackages.pnpm
      nodejs
      bun
    ];

    sessionPath = ["$HOME/.npm-global/bin"];

    file.".npmrc".text = ''
      prefix=${config.home.homeDirectory}/.npm-global
    '';
  };
}
