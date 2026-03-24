{
  config,
  pkgs,
  lib,
  ...
}: let
  NPM_GLOBAL_DIR = "${config.home.homeDirectory}/.npm-global";
  BUN_GLOBAL_DIR = "${config.home.homeDirectory}/.bun-global";
in {
  home = {
    packages = with pkgs; [
      npm-check-updates # (ncu) Find newer versions of package dependencies and check outdated npm packages locally or globally.
      npkill # remove node_modules from child directories
      nest-cli
      husky
      biome
    ];

    sessionVariables = {
      # https://github.com/npm/cli/issues/7857#issuecomment-2481331001
      NODE_OPTIONS = "--disable-warning=ExperimentalWarning";
    };

    sessionPath = ["${NPM_GLOBAL_DIR}/bin" "${BUN_GLOBAL_DIR}/bin"];

    activation.init = lib.hm.dag.entryAfter ["writeBoundary"] ''
      mkdir -p {${NPM_GLOBAL_DIR},${BUN_GLOBAL_DIR}}/{bin,lib}
    '';
  };

  programs.npm = {
    enable = true;
    package = pkgs.nodejs_25;
    settings = {
      audit = true;
      audit-level = "critical";
      fund = false;
      ignore-scripts = true;
      init-private = true;
      init-version = "0.0.1";
      init-author-email = config.programs.git.settings.user.email;
      init-author-name = config.programs.git.settings.user.name;
      init-license = "MIT";
      prefix = NPM_GLOBAL_DIR;
    };
  };

  programs.bun = {
    enable = true;
    settings = {
      runtime = {
        logLevel = "debug";
        telemetry = false;
      };

      install = {
        optional = true;
        dev = true;
        peer = true;
        production = false;
        exact = true;
        globalBinDir = "${BUN_GLOBAL_DIR}/bin";
        cache = "${BUN_GLOBAL_DIR}/cache";
      };

      console = {
        depth = 3;
      };

      auto = "fallback";
    };
  };
}
