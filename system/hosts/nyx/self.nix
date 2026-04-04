{
  config,
  sys,
  ...
}: {
  programs.self = {
    enable = true;

    commands = {
      update = {
        description = "Updates your computer using your system and/or home configuration";

        steps = sys.compose "update" ["system" "home"];

        subcommands = {
          system = {
            steps = [
              sys.requireSudo
              (sys.log "Updating system...")
              (sys.run "sudo nixos-rebuild switch --flake .#${config.networking.hostName}")
            ];
          };

          home = {
            steps = [
              (sys.log "Updating home...")
              (sys.run "home-manager switch --flake .")
            ];
          };
        };
      };

      clean = {
        description = "Cleans with the old generations";

        steps = [
          (sys.run "nix-collect-garbage -d")
        ];
      };

      style = {
        description = "Applies alejandra style to all .nix files";

        steps = [
          (sys.run "alejandra .")
        ];
      };
    };
  };
}
