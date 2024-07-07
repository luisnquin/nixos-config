{
  config,
  pkgs,
  lib,
  ...
}: let
  npmGlobalDir = "${config.home.homeDirectory}/.npm-global";
in {
  home = {
    packages = with pkgs; [
      nodePackages.pnpm
      nodejs
      bun
    ];

    sessionPath = ["${npmGlobalDir}/bin"];

    file.".npmrc".text = ''
      prefix=${npmGlobalDir}
    '';
  };

  home.activation.init = lib.hm.dag.entryAfter ["writeBoundary"] ''
    mkdir -p ${npmGlobalDir}/bin \
             ${npmGlobalDir}/lib
  '';
}
