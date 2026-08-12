#!/bin/sh

setup() {
	DOTS_DIR="$HOME/.dotfiles"
	git clone https://github.com/luisnquin/dotfiles.git "$DOTS_DIR"

	cd "$DOTS_DIR"
	nix --experimental-features "nix-command flakes" \
		run github:nix-community/disko -- --mode disko ./system/hosts/nyx/disko-config.nix

	nixos-install --root /mnt --flake github:luisnquin/nixos-config#nyx
}

main() {
	sudo sh -c "$(declare -f setup); setup"
}

main
