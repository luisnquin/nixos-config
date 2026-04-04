{
  hmConfig,
  config,
  sys,
  ...
}: let
  dotfilesPath = "${hmConfig.home.homeDirectory}/.dotfiles";
in {
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
              (sys.run {
                workdir = dotfilesPath;
                cmd = "sudo nixos-rebuild switch --flake .#${config.networking.hostName}";
              })
            ];
          };

          home = {
            steps = [
              (sys.run {
                workdir = dotfilesPath;
                cmd = "home-manager switch --flake .";
              })
            ];
          };
        };
      };

      clean = {
        description = "Cleans with the old generations";

        steps = [
          sys.requireSudo
          (sys.run "sudo nix-collect-garbage --delete-old")
          (sys.run "nix-collect-garbage -d")
          (sys.run "nix-store --delete")
          (sys.run "rm -rf ~/.npm/_npx")
          (sys.run "docker system prune --volumes -f")
          (sys.run "kondo -a")
        ];
      };

      style = {
        description = "Applies alejandra style to all .nix files";

        steps = [
          (sys.run {
            workdir = dotfilesPath;
            cmd = "alejandra .";
          })
        ];
      };
    };
  };
}
