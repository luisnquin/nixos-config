{
  pkgs-extra,
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
      nodePackages.pnpm
      pkgs-extra.npkill # remove node_modules from child directories
      nest-cli
      pkgs.nodejs_24
      husky
      pkgs.biome
      bun
    ];

    sessionVariables = {
      # https://github.com/npm/cli/issues/7857#issuecomment-2481331001
      NODE_OPTIONS = "--disable-warning=ExperimentalWarning";
    };

    sessionPath = ["${npmGlobalDir}/bin"];

    activation.init = lib.hm.dag.entryAfter ["writeBoundary"] ''
      mkdir -p ${npmGlobalDir}/bin \
               ${npmGlobalDir}/lib
    '';

    file = {
      ".npmrc".text = ''
        prefix=${npmGlobalDir}
        //registry.npmjs.org/:_authToken=''${NPM_TOKEN}
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
