{
  spicetify-nix,
  pkgs,
  lib,
  ...
}: let
  owner = import ../owner.nix;
  spicePkgs = spicetify-nix.packages.${pkgs.system}.default;
in {
  imports = [
    ./modules/default.nix
  ];

  home = {
    stateVersion = "23.05";
    enableNixpkgsReleaseCheck = true;
    homeDirectory = "/home/luisnquin";
    username = "luisnquin";
  };

  programs.zsh = {
    enable = true;
    enableCompletion = true;
    completionInit = ''
      autoload -U compinit && compinit
      source <(nao completion zsh); compdef _nao nao
      complete -C "$(which aws_completer)" aws

      # Displays only Makefile rules unless there arent'
      zstyle ':completion::complete:make::' tag-order targets variables
    '';
    # TODO: improve nao completions

    plugins = with pkgs; [
      {
        name = "nix-shell";
        file = "nix-shell.plugin.zsh";
        src = fetchFromGitHub {
          owner = "chisui";
          repo = "zsh-nix-shell";
          rev = "v0.5.0";
          sha256 = "0za4aiwwrlawnia4f29msk822rj9bgcygw6a8a6iikiwzjjz0g91";
        };
      }
      {
        name = "you-should-use";
        file = "you-should-use.plugin.zsh";
        src = fetchFromGitHub {
          owner = "MichaelAquilina";
          repo = "zsh-you-should-use";
          rev = "5b316f4af3ac90e044f386003aacdaa0ad606488";
          sha256 = "192jb680f1sc5xpgzgccncsb98xa414aysprl52a0bsmd1slnyxs";
        };
      }
      {
        name = "extract";
        file = "extract.sh";
        src = fetchFromGitHub {
          owner = "xvoland";
          repo = "Extract";
          rev = "439e92c5b355b40c36d8a445636d0e761ec08217";
          sha256 = "1yaphcdnpxcdrlwidw47waix8kmv2lb5a9ccmf8dynlwvhyvh1wi";
        };
      }
      {
        name = "zinsults";
        file = "zinsults.plugin.zsh";
        src = fetchFromGitHub {
          owner = "ahmubashshir";
          repo = "zinsults";
          rev = "2963cde1d19e3af4279442a4f67e4c0224341c42";
          sha256 = "1mlb0zqaj48iwr3h1an02ls780i2ks2fkdsb4103aj7xr8ls239b";
        };
      }
    ];
  };

  xdg = {
    enable = true;
    configFile = with owner; {
      "go/env".text = builtins.readFile ../../dots/home/go/env;

      "openaiapirc".text = ''
        [openai]
        organization_id = ${openai.organization-id}
        secret_key = ${openai.secret-key}
      '';

      "ai/.ai-cli".text = ''
        OPENAI_API_KEY=${openai.secret-key}
      '';

      "rclone/rclone.conf".text = builtins.concatStringsSep "\n" (builtins.attrValues rclone);
      "alacritty.yml".text = builtins.readFile ../../../dots/home/alacritty.yml;
      "k9s/views.yml".text = builtins.readFile ../../dots/home/k9s/views.yml;
      "k9s/skin.yml".text = builtins.readFile ../../dots/home/k9s/skin.yml;
    };
  };

  programs = {
    alacritty = {
      enable = true;
      package = pkgs.alacritty;
      # Let Home Manager install and manage itself.
    };
    home-manager.enable = true;
    vscode = {
      enable = true;
      enableExtensionUpdateCheck = true;
      enableUpdateCheck = true;
      package = pkgs.vscode;
      # extensions = with vscode-extensions; [
      #   matklad.rust-analyzer
      #   jnoortheen.nix-ide
      # ];
      mutableExtensionsDir = true;
    };
  };

  #environment.systemPackages = with pkgs; [
  #  spotify
  #];

  # allow spotify to be installed if you don't have unfree enabled already
  #nixpkgs.config.allowUnfreePredicate = pkg:
  #  builtins.elem (lib.getName pkg) [
  #    "spotify"
  #  ];

  imports = [
    inputs.spicetify-nix.homeManagerModule
  ];

  # configure spicetify :)
  programs.spicetify = {
    enable = true;
    theme = spicePkgs.themes.catppuccin-mocha;
    colorScheme = "flamingo";

    enabledExtensions = with spicePkgs.extensions; [
      fullAppDisplay
      shuffle # shuffle+ (special characters are sanitized out of ext names)
      hidePodcasts
    ];
  };
}
