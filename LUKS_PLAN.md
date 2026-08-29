# LUKS2 + TPM2 auto-unlock plan

Goal: encrypt `nvme0n1p3` (the everything-partition: /persist, /nix, /home)
and have the TPM release the key only when the boot chain is the one we
signed. Secure Boot (done 2026-08-29) is the anchor; this is the payoff.

End state: power on → signed UKI boots → TPM checks PCR 7 (secure boot
policy + our keys) → disk unlocks with no passphrase → tmpfs root as today.
Steal the laptop, tamper with the boot chain, or toggle SB off → TPM refuses
→ passphrase prompt. ESP stays plaintext by design; the signatures are what
make that safe.

## Phase 0 — backups (blocking prerequisite)

Nothing touches the disk until all of this exists off-machine:

- [ ] Full copy of `/persist` and `/home` to an external disk
      (one partition: everything lives on p3)
- [ ] The three roots of trust:
      1. `/etc/ssh/ssh_host_ed25519_key` — sops decryption root; losing it
         means rekeying every secret from another trusted machine
      2. `/persist/var/lib/sbctl` — secure boot signing keys; losing them
         means redoing the firmware enrollment dance
      3. `~/.ssh/id_ed25519` — user identity; replaceable, convenience
- [ ] A NixOS live USB, tested to boot on this machine (SB off toggle or a
      signed installer)

## Phase 1 — systemd initrd (repo-only, do first, standalone value)

TPM2 unlock exists only in the systemd-based initrd, not the legacy script
one. This lands and soaks BEFORE any encryption:

- `boot.initrd.systemd.enable = true`
- Verify the preservation `inInitrd` entries (machine-id, host key) still
  bind — preservation is built for systemd initrd, but prove it
- Reboot, check sops secrets decrypt, journal clean

Fully reversible commit. If anything is weird, fix it here where the disk
is still plaintext.

## Phase 2 — encryption day (offline, from the live USB)

Route: **in-place reencrypt** — no reformat, no restore, data stays put.
(The backup from Phase 0 is the safety net, not part of the happy path.)

From the live USB, disk unmounted:

```sh
# make room for the LUKS2 header (32M) at the end of the fs
e2fsck -f /dev/nvme0n1p3
resize2fs /dev/nvme0n1p3 <current_blocks - 32M worth>

# encrypt in place; takes on the order of 1-2h for the full partition
cryptsetup reencrypt --encrypt --reduce-device-size 32M /dev/nvme0n1p3
```

Passphrase chosen here is the recovery credential — long, written down,
stored with the Phase 0 backups. Then, same session:

```sh
cryptsetup luksHeaderBackup /dev/nvme0n1p3 --header-backup-file <external-disk>
```

Header backup is non-negotiable: a corrupted header with no backup equals
all data gone regardless of passphrase.

Repo changes (same commit, applied via the live USB chroot or first boot):

- `disko-config.nix`: wrap the `root` partition content in
  `type = "luks"; name = "cryptroot"` with the ext4 inside — keeps the
  fresh-install path (`pkgs/setup`) producing the same layout
- `fileSystems`/`boot.initrd.luks.devices.cryptroot` come out of disko
- Verify `neededForBoot` semantics: /persist must open in initrd (it
  already does — preservation depends on it)

Boot test: passphrase prompt appears, system comes up exactly as before.
Live on passphrase-at-boot until Phase 3 feels earned.

## Phase 3 — TPM enrollment (on the running system, 5 minutes)

```sh
sudo systemd-cryptenroll /dev/nvme0n1p3 \
  --tpm2-device=auto --tpm2-pcrs=7
```

- PCR 7 only: measures secure boot state + key set, survives kernel and
  NixOS updates untouched. PCR 11/pcrlock is stronger but operationally
  churny — not for round one.
- Decide at the console: add `--tpm2-with-pin=yes` if a thief reaching the
  login screen bothers you; without it, disk security at rest degrades to
  the user password once the machine is stolen whole.
- Config: `boot.initrd.luks.devices.cryptroot.crypttabExtraOpts =
  [ "tpm2-device=auto" ]`
- The passphrase keyslot STAYS. Forever. TPM slot is convenience.

## Phase 4 — drills (proves the design, 20 minutes)

- [ ] Normal reboot: no passphrase asked
- [ ] Firmware → SB off → boot: TPM must refuse, passphrase prompt works,
      SB back on, auto-unlock returns  ← this drill IS the security model
- [ ] Old generation from the boot menu still unlocks
- [ ] `cryptsetup luksDump` shows exactly 2 keyslots (passphrase + tpm2)

## The other shit (what makes the whole stack coherent)

- [ ] ESP cleanup, GRUB leftovers (~120M) — the last unsigned binaries
      on the disk, doable today:
      `sudo rm -r /boot/grub /boot/kernels /boot/theme /boot/background.png /boot/converted-font.pf2`
- [ ] Firmware supervisor/admin password — otherwise anyone can toggle SB
      off or reorder boot; with LUKS that no longer exposes data, but it
      enables evil-maid swaps of the whole chain. Cheap, do it.
- [ ] User password strength review — with auto-unlock it becomes the
      real perimeter (or take the TPM PIN instead)
- [ ] Firmware updates: expect PCR 7 to change → passphrase prompt once →
      `systemd-cryptenroll --wipe-slot=tpm2` + re-enroll. Routine, not
      an incident
- [ ] sops-wire the sbctl keys + user ssh key so backups reduce to the
      host key + LUKS passphrase + header file
- [ ] swap: none exists today (`nohibernate` already set) — if swap is
      ever added it goes inside the LUKS volume, never beside it

## Rollback map

- Phase 1: revert commit, rebuild
- Phase 2 failed mid-reencrypt: `cryptsetup reencrypt` resumes if
  interrupted; disk hosed = restore Phase 0 backup onto a fresh luksFormat
- Phase 3: `systemd-cryptenroll --wipe-slot=tpm2`, back to passphrase-only
- Any boot refusal, any phase: live USB + passphrase opens the disk;
  worst case is the Phase 0 restore

## Standing notes (carried from the secure boot work)

- `sbctl verify` always flags `EFI/nixos/kernel-*.efi` unsigned: expected,
  the signed stub hash-verifies kernel+initrd
- TPM NV space can't fit systemd's `login` NvPCR → journal error each
  boot; harmless, unrelated to the PCR 7 binding here
- Escape hatches, forever: firmware SB toggle off always boots; "Restore
  default keys" reverts to factory Microsoft state
