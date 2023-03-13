# NixOS environment configuration

 [![nixos-unstable](https://img.shields.io/badge/NixOS-unstable-informational.svg?style=flat&logo=nixos&logoColor=dee1e6&colorA=101419&colorB=70a5eb)](https://github.com/nixos/nixpkgs)
[![nix-fmt](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml/badge.svg)](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml)

⚠ This is a single user setup and is not intended to be anything else

## Installation

```bash
# Nix environment setup + nyx computer manager
$ curl -s https://raw.githubusercontent.com/luisnquin/nixos-config/main/setup.sh | sh
```

## Nyx manager

```sh
# A sh script focused in NixOS
$ nyx --help

nyx [command] [flags]

Available commands:
  update ⛄    Updates the machine
  inspect      Verifies if the configuration.nix file has been changed and not saved to a git repository
  style 💅     Applies alejandra style to all .nix files
  ls           List elements in dotfiles directory
  clean        Cleans with the old generations

Global flags:
-h, --help Print help information
```

## My current computer

TODO: update screenshoot

![image](https://user-images.githubusercontent.com/86449787/183443225-e7442ddf-ab0f-47d1-b712-68a6d1d669c6.png)

Check [here](https://nmikhailov.github.io/nixpkgs/ch-options.html) to see more options.

## Some configurations comes here thanks to

- [angristan](https://github.com/angristan/nixos-config)
- [kmein](https://github.com/kmein/niveum)
- [mogria](https://github.com/mogria/nixpkgs-config)
- [qbit](https://github.com/qbit/nix-conf)
- [rxyhn](https://github.com/rxyhn/dotfiles)
