# NixOS environment configuration

 [![nixos-unstable](https://img.shields.io/badge/NixOS-unstable-informational.svg?style=flat&logo=nixos&logoColor=dee1e6&colorA=101419&colorB=70a5eb)](https://github.com/nixos/nixpkgs)
[![nix-fmt](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml/badge.svg)](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml)

## Usage

### Installation

```bash
# Nix environment setup + nyx computer manager
$ sh <(curl -s https://raw.githubusercontent.com/luisnquin/nixos-config/main/.scripts/init.sh)
```

### Manage updates with

```bash
# Updates the computer using the .nix files
$ nyx update
```

### See more with

```bash
# Displays help
$ nyx --help
```

_Change the alias for the name of your machine in_ `configuration.nix`.

## My computer

![image](https://user-images.githubusercontent.com/86449787/183443225-e7442ddf-ab0f-47d1-b712-68a6d1d669c6.png)

Check [here](https://nmikhailov.github.io/nixpkgs/ch-options.html) to see more options.

## Inspired by

- [angristan](https://github.com/angristan/nixos-config)
- [kmein](https://github.com/kmein/niveum)
- [mogria](https://github.com/mogria/nixpkgs-config)
- [qbit](https://github.com/qbit/nix-conf)
- [rxyhn](https://github.com/rxyhn/dotfiles)
