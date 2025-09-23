# NixOS environment configuration

[![nixos-unstable](https://img.shields.io/badge/NixOS-unstable-informational.svg?style=flat&logo=nixos&logoColor=dee1e6&colorA=101419&colorB=70a5eb)](https://github.com/nixos/nixpkgs)
[![nix-fmt](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml/badge.svg)](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml)

> **Warning**
>
> Single user setup and is not intended to be anything else so fo

## Setup

No GUI or "manual steps" are required so just get the minimal ISO (if possible).

```bash
# Clone the configuration (or just grab the disko-config.nix file)
$ git clone https://github.com/luisnquin/nixos-config.git

# Partition and format disks with disko
$ sudo nix --experimental-features "nix-command flakes" run \
    github:nix-community/disko -- --mode disko nixos-config/system/hosts/nyx/disko-config.nix

# Install NixOS using the configuration flake
$ sudo nixos-install --root /mnt --flake github:luisnquin/nixos-config#nyx
```

After that just reboot and continue the setup with home manager.

## How does it look like?

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/dc8cea0a-d10d-4eb1-bc52-ebfc809c2db1" />
