{ config, pkgs, ... }:

{
  imports = [ # Include the results of the hardware scan.
    ./hardware-configuration.nix
  ];

  # Use the systemd-boot EFI boot loader.
  boot.loader.efi.canTouchEfiVariables = true;
  boot.loader.systemd-boot.enable = true;

  system.autoUpgrade.enable = true;
  system.autoUpgrade.channel = "https://nixos.org/channels/nixos-22.05";

  networking = {
    networkmanager.enable = true;
    hostName = "nyx";

    # Enables wireless support via wpa_supplicant.
    # wireless.enable = true;
  };

  time.timeZone = "America/Lima";

  networking = {
    firewall = {
      enable = true;
      allowedTCPPorts = [ 80 443 8000 8080  ];
    };

    enableIPv6 = true;
    useDHCP = false;

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
    # Enable the X11 windowing system.
    enable = true;

    # Configure keymap in X11
    layout = "latam";

    # Enable the Plasma 5 Desktop Environment.
    displayManager.sddm.enable = true;
    desktopManager.plasma5.enable = true;
  };

  sound.enable = true;

  hardware = {
    pulseaudio.enable = true;
    nvidia.package = config.boot.kernelPackages.nvidiaPackages.stable;
    opengl.enable = true;
  };

  users.users.luisnquin = {
    isNormalUser = true;
    shell = pkgs.zsh;
    # password = "";

    extraGroups = [ "wheel" "docker" ];
  };

  nixpkgs.config.allowUnfree = true;

  services.xserver.displayManager.startx.enable = true;
  services.xserver.desktopManager.xterm.enable = true;

  services.xserver = {
    autorun = true;

    videoDrivers = [ "intel" "modesetting" ];

    # Enable touchpad support (enabled default in most desktopManager).
    libinput.enable = true;
  };

  environment.systemPackages = with pkgs; [
    # golangci-lint
    gcc
    g++

    python310
    virtualenv

    nodejs-18_x
    #npm

    docker-compose
    docker

    postgresql

    nixfmt
    git

    spotify
    discord
    vscode
    slack

    # etc
    redoc-cli
    binutils
    gnumake
    openjdk
    openssh
    unzip
    wget
    dpkg
    tree
    stow
    tmux
    # cron
    zip
    zsh
  ];

  fonts.fonts = with pkgs; [ cascadia-code jetbrains-mono ];

  programs.mtr.enable = true;

  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };

  programs.tmux = {
    enable = true;
    clock24 = true;
    newSession = true;
  };

  virtualisation.docker.enable = true;

  services.openssh.enable = true;

  programs.zsh.enable = true;

  environment.sessionVariables = rec {
    # Go definitions
    GOPRIVATE = "gitlab.com/wiserskills/";
    PATH = "$GORROT:$GOPATH/bin:$PATH";
    GO111MODULE = "on";
  };

  environment.interactiveShellInit = ''
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

  systemd.services.batteryChargeThreshold = {
    enable = true;
    description = "Set the battery charge threshold";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = ''${pkgs.bash}/bin/bash -c "echo 61 > /sys/class/power_supply/BAT1/charge_control_end_threshold"'';
      ExecStop = ''${pkgs.bash}/bin/bash -c "exit 0"'';
    };
    wantedBy = [ "multi-user.target" ];
  };

  system.stateVersion = "21.11";
}
