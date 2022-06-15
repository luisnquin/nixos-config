{ config, pkgs, ... }:

{
  imports = [ # Include the results of the hardware scan.
    ./hardware-configuration.nix
  ];

  nixpkgs.config.allowUnfree = true;

  time.timeZone = "America/Lima";

  fonts.fonts = with pkgs; [ cascadia-code jetbrains-mono nerdfonts ];

  # Use the systemd-boot EFI boot loader.
  boot.loader = {
    efi.canTouchEfiVariables = true;
    systemd-boot.enable = true;

    grub = {
      enable = true;
      version = 2;
      device = "nodev";
      useOSProber = true;
      efiSupport = true;
      # extraConfig = "settheme=${pkgs.plasma5.breeze-grub}/grub/themes/breeze/theme.txt";
    };
  };

  services.openssh.enable = true;
  # xdg.portal.wlr.enable = true;
  virtualisation.docker.enable = true;

  system.autoUpgrade = {
    enable = true;
    allowReboot = true;
    channel = "https://nixos.org/channels/nixos-22.05";
  };

  security.sudo = {
    enable = true;
    wheelNeedsPassword = false;
  };

  networking = {
    # useNetworkd = true;
    # dhcpcd.enable = false;
    networkmanager.enable = true;
    hostName = "nyx";

    wireless.enable = false;
    # wireless.iwd.enable = true;

    firewall = {
      enable = true;
      allowedTCPPorts = [ 80 443 ];
      allowPing = false;
    };

    enableIPv6 = true;
    useDHCP = false;

    # interfaces.eth0.useDHCP = true;
    # interfaces.wlan0.useDHCP = true; 

    interfaces = {
      enp4s0.useDHCP = false;
      wlp3s0.useDHCP = false;
      wlan0.useDHCP = false;
    };
  };

  i18n.defaultLocale = "es_PE.UTF-8";
  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  services.xserver = {
    enable = true;

    layout = "latam";

    displayManager.sddm.enable = true;
    desktopManager.plasma5.enable = true;
  };

  sound.enable = true;

  hardware = {
    pulseaudio.enable = true;
    opengl.enable = true;
    # nvidia = {
    #   package = config.boot.kernelPackages.nvidiaPackages.stable;
    #   powerManagement.enable = true;
    #   modesetting.enable = true;
    # };
  };

  users.users.luisnquin = {
    isNormalUser = true;
    home = "/home/luisnquin";
    description = "Luis Quiñones";
    shell = pkgs.zsh;

    extraGroups = [ "wheel" "docker" "adbusers" ];
  };

  services.xserver.displayManager.startx.enable = true;
  services.xserver.desktopManager.xterm.enable = true;

  services.xserver = {
    autorun = true;
    videoDrivers = [ "intel" "modesetting" ];
    libinput.enable = true;
  };

  environment.systemPackages = with pkgs; [
    go
    # gopls 
    gcc

    python310
    virtualenv

    nodejs-18_x
    #npm

    # Android development
    android-tools
    flutter
    dart

    docker-compose
    docker

    postgresql

    nixfmt
    git

    spotify
    discord
    vscode
    slack

    redoc-cli
    pre-commit
    binutils
    gnumake
    openjdk
    openssh
    unzip
    sass
    wget
    dpkg
    tree
    stow
    tmux
    bat
    # cron
    zip
    zsh
    jq
  ];

  programs = {
    adb.enable = true;
    mtr.enable = true;

    tmux = {
      enable = true;
      clock24 = true;
      newSession = true;
      historyLimit = 50000;
    };

    gnupg.agent = {
      enable = true;
      enableSSHSupport = true;
    };

    zsh = {
      enable = true;
      autosuggestions.enable = true;
      enableCompletion = true;
      syntaxHighlighting.enable = true;
      # sway.enable = true;
      # dotDir = ".config/zsh";
      # plugins = [
      #   {
      #     name = "zsh-you-should-use";
      #     src = pkgs.zsh-you-should-use;
      #    }
      # ];
    };

  };

  environment = {
    sessionVariables = rec {
      GOPRIVATE = "gitlab.com/wiserskills/";
      PATH = "$GORROT:$GOPATH/bin:$PATH";
      GO111MODULE = "on";
    };

    interactiveShellInit = ''
      alias dataserver='cd ~/go/src/gitlab.com/wiserskills/v3/dataserver'

      alias playground='cd ~/workspace/playground'
      alias projects='cd ~/workspace/projects'
      alias tests='cd ~/workspace/tests'
      alias workspace='cd ~/workspace'

      alias edu='cd ~/.education'
      alias work='cd ~/.work'
      alias etc='cd ~/.etc'

      alias open='xdg-open'
      alias py='python3'
    '';
  };

  systemd.services.batteryChargeThreshold = {
    enable = true;
    description = "Set the battery charge threshold";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''
        ${pkgs.bash}/bin/bash -c "echo 61 > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
      ExecStop = ''${pkgs.bash}/bin/bash -c "exit 0"'';
    };
    wantedBy = [ "multi-user.target" ];
  };

  system.stateVersion = "21.11";
}
