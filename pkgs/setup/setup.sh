#!/bin/sh

setup() {
	DOTS_DIR="$HOME/.dotfiles"
	git clone https://github.com/luisnquin/dotfiles.git "$DOTS_DIR"

	cd "$DOTS_DIR"
	nix --experimental-features "nix-command flakes" \
		run github:nix-community/disko -- --mode disko ./system/hosts/nyx/disko-config.nix

	# a /var/lib/sbctl restored from backup keeps the enrolled secure boot
	# keys; fresh ones need the firmware enrollment dance again
	[ -d /var/lib/sbctl ] ||
		nix --experimental-features "nix-command flakes" run nixpkgs#sbctl -- create-keys
	mkdir -p /mnt/persist/var/lib /mnt/var/lib
	cp -a /var/lib/sbctl /mnt/persist/var/lib/
	cp -a /var/lib/sbctl /mnt/var/lib/ # lzbt runs in the chroot before preservation binds exist

	nixos-install --root /mnt --flake github:luisnquin/nixos-config#nyx
}

main() {
	sudo sh -c "$(declare -f setup); setup"
}

main
