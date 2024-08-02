{
  config,
  pkgsx,
  pkgs,
  lib,
  ...
}: {
  home = let
    npmGlobalDir = "${config.home.homeDirectory}/.npm-global";
  in {
    packages = with pkgs; [
      npm-check-updates # (ncu) Find newer versions of package dependencies and check outdated npm packages locally or globally.
      nodePackages."@angular/cli"
      nodePackages.pnpm
      pkgsx.npkill # remove node_modules from child directories
      nest-cli
      nodejs
      biome
      bun
    ];

    sessionPath = ["${npmGlobalDir}/bin"];

    file.".npmrc".text = ''
      prefix=${npmGlobalDir}
    '';

    activation.init = lib.hm.dag.entryAfter ["writeBoundary"] ''
      mkdir -p ${npmGlobalDir}/bin \
               ${npmGlobalDir}/lib
    '';
  };
}
