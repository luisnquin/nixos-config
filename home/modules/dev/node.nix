{
  config,
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
      pkgs.extra.npkill # remove node_modules from child directories
      nodejs_23
      nest-cli
      husky
      biome
      bun
    ];

    sessionPath = ["${npmGlobalDir}/bin"];

    activation.init = lib.hm.dag.entryAfter ["writeBoundary"] ''
      mkdir -p ${npmGlobalDir}/bin \
               ${npmGlobalDir}/lib
    '';

    file = {
      ".npmrc".text = ''
        prefix=${npmGlobalDir}
      '';

      ".bunfig.toml".text = ''
        [runtime]
        logLevel = "debug"
        telemetry = false

        [install]
        optional = true
        dev = true
        peer = true
        production = false
        exact = true

        auto = "fallback"
      '';
    };
  };
}
