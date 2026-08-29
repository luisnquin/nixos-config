# NixOS environment configuration

[![nixos-unstable](https://img.shields.io/badge/NixOS-unstable-informational.svg?style=flat&logo=nixos&logoColor=dee1e6&colorA=101419&colorB=70a5eb)](https://github.com/nixos/nixpkgs)
[![nix-fmt](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml/badge.svg)](https://github.com/luisnquin/nixos-config/actions/workflows/style.yml)

> **Warning**
>
> Single user setup and is not intended to be anything else so fo

## Setup

No GUI or "manual steps" are required so just get the minimal ISO (if possible).

```bash
# Partitions and formats with disko, provisions the secure boot signing
# keys and installs the flake
$ nix --experimental-features "nix-command flakes" run github:luisnquin/nixos-config#setup
```

After that just reboot and continue the setup with home manager. To keep the
firmware's secure boot enrollment, restore `/var/lib/sbctl` from backup before
running the script; with fresh keys, re-enroll once after first boot: firmware →
delete PK (Setup Mode), `sbctl enroll-keys --microsoft`, enable secure boot.

## How does it look like?

<img width="1920" height="1080" alt="image" src="https://github.com/user-attachments/assets/c188cc0a-9e9b-448f-b999-f28dfbc83ad9" />
