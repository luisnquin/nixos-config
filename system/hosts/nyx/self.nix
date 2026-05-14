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

        steps = sys.withNotify {
          successTitle = "Nyx update finished";
          successBody = "NixOS and Home Manager applied.";
          failureTitle = "Nyx update failed";
          failureBody = "Check the terminal output for the failing step.";
        } (sys.compose "update" ["system" "home"]);

        subcommands = {
          system = {
            steps =
              sys.withNotify {
                successTitle = "Nyx system update finished";
                successBody = "NixOS configuration applied.";
                failureTitle = "Nyx system update failed";
                failureBody = "nixos-rebuild did not complete successfully.";
              } [
                sys.requireSudo
                (sys.run {
                  workdir = dotfilesPath;
                  cmd = "sudo nixos-rebuild switch --flake .#${config.networking.hostName}";
                })
              ];
          };

          home = {
            steps =
              sys.withNotify {
                successTitle = "Nyx home update finished";
                successBody = "Home Manager generation applied.";
                failureTitle = "Nyx home update failed";
                failureBody = "home-manager switch did not complete successfully.";
              } [
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
