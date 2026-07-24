# Secure Boot plan (lanzaboote)

> [!NOTE]
> Planning doc, not yet executed. Goal: give `nyx` a signed boot chain via
> [`nix-community/lanzaboote`](https://github.com/nix-community/lanzaboote).
> **Execute alongside / right after [`IMPERMANENCE_PLAN.md`](./IMPERMANENCE_PLAN.md)** —
> the two overlap in two concrete places (ESP sizing during the disko rewrite,
> and persisting the signing keys past the boot-wipe). Doing them in one window
> avoids a second reinstall-adjacent scare.

## TL;DR

- **Mechanism:** lanzaboote replaces systemd-boot, bundling kernel + initrd +
  cmdline into a signed **UKI** (Unified Kernel Image) per generation, stored on
  the ESP. Firmware verifies the signature against keys you enrolled.
- **Blocker:** `nyx` boots **GRUB** today. lanzaboote is systemd-boot–only and
  incompatible with GRUB. Migration path is GRUB → systemd-boot → lanzaboote,
  each step verified before the next.
- **Two hard dependencies on the impermanence migration:**
  1. **ESP is 500M — too small.** UKIs are far larger than systemd-boot entries
     (whole initrd baked in), and with the NVIDIA initrd + multiple generations
     500M fills fast. The impermanence rewrite already recreates the partition
     table → **bump ESP to 2G there**, and drop the dead `EF02` grub-MBR part.
  2. **Signing keys must survive the wipe.** `/etc/secureboot` holds the private
     keys used to re-sign on every `nixos-rebuild`. On an ephemeral root it must
     be in the persist set, or the next rebuild can't sign.
- **Cost:** flake input + one boot module swap + a one-time firmware dance
  (Setup Mode → enroll → enable SB). Fully reversible from firmware (toggle SB
  off). No data risk on its own — the data risk lives in the impermanence doc.

## Why pair it with impermanence (not before, not standalone)

| Concern | If done standalone | If folded into impermanence |
| --- | --- | --- |
| ESP 500M → 2G | needs a partition resize (risky in place) | free — disko recreates the table anyway |
| `EF02` 1M grub stub | left as dead weight | dropped in the btrfs layout rewrite |
| Key persistence | N/A (root is persistent) | **required** — `/etc/secureboot` on `/persist` |
| Failure surface | isolated | isolate by ordering: SB enrollment strictly **after** impermanence first-boot is verified clean |

> [!IMPORTANT]
> Decide the ESP size and partition layout **during** impermanence Phase 3
> (disko rewrite) — you cannot cheaply resize the ESP later. The Secure Boot
> *enrollment* happens **after** the impermanence reinstall boots clean.

## Prerequisites

- [ ] UEFI firmware (not CSM/legacy) — `nyx` already boots EFI GRUB, so yes.
- [ ] Firmware supports **custom key enrollment** / a **Setup Mode** (clear PK).
      Check the BIOS "Secure Boot" menu for "Erase all keys" / "Setup Mode".
- [ ] Secure Boot currently **OFF** (it is — GRUB unsigned).
- [ ] A NixOS Live USB on hand (shared with the impermanence migration).
- [ ] Know your firmware key combo (Del/F2) — you may need to toggle SB off to
      recover.

## Phase plan

Ordered so each phase is independently bootable and reversible.

### Phase A — ESP + layout (folded into impermanence Phase 3)

In the btrfs disko rewrite (`system/hosts/nyx/disko-config.nix`):

- Drop the `boot` `EF02` 1M partition (grub-only, dead once systemd-boot lands).
- Grow `ESP` from `500M` → **`2G`**, keep `EF00` / vfat / `mountpoint = "/boot"`.

```
nvme0n1
├─ p1  EF00   2G     /boot   (vfat, ESP — holds signed UKIs)
└─ p2  100%          btrfs   (@, @nix, @persist, @swap … per impermanence doc)
```

> No separate step to execute here — just make the two edits when you rewrite
> disko for impermanence. Everything below runs on the freshly reinstalled box.

### Phase B — GRUB → systemd-boot (unsigned, verify boot)

Boot module swap (`system/modules/boot/`):

- Delete `grub.nix`, replace with `systemd-boot.nix`, update `default.nix` imports.

```nix
# system/modules/boot/systemd-boot.nix
{...}: {
  boot.loader = {
    systemd-boot = {
      enable = true;
      configurationLimit = 20; # UKIs are big; cap generations on a 2G ESP
      consoleMode = "max";
    };
    efi.canTouchEfiVariables = true; # was false (grub used efiInstallAsRemovable)
  };
}
```

- `nixos-rebuild boot`, reboot, confirm systemd-boot menu appears and boots.
  **Secure Boot still off.** This isolates the bootloader swap from signing.

> [!NOTE]
> `useOSProber` is gone with GRUB. systemd-boot auto-discovers other EFI loaders
> (e.g. Windows) on the ESP — if you dual-boot, confirm the entry still shows.

### Phase C — lanzaboote (sign, still SB-off)

1. **Flake input** — mirror the existing input wiring in `flake.nix`:
   ```nix
   lanzaboote = {
     url = "github:nix-community/lanzaboote/v0.4.2";
     inputs = {
       nixpkgs.follows = "nixpkgs";
       flake-utils.follows = "flake-utils";
     };
   };
   ```
   Add to the `nixosModules` list (next to `disko.nixosModules.default`):
   ```nix
   inputs.lanzaboote.nixosModules.lanzaboote
   ```

2. **Boot module** — turn `systemd-boot.nix` into the lanzaboote handoff.
   lanzaboote *needs* systemd-boot present but takes over installation, so force
   its installer off:
   ```nix
   { lib, pkgs, ... }: {
     environment.systemPackages = [ pkgs.sbctl ]; # verify/sign/enroll CLI

     boot.loader.systemd-boot.enable = lib.mkForce false;
     boot.loader.efi.canTouchEfiVariables = true;

     boot.lanzaboote = {
       enable = true;
       pkiBundle = "/etc/secureboot"; # MUST match the persisted path (Phase E)
     };
   }
   ```

3. **Generate keys, before enabling SB in firmware:**
   ```sh
   sudo sbctl create-keys        # writes /etc/secureboot/{keys,GUID}
   sudo nixos-rebuild boot       # lanzaboote signs the UKI(s)
   ```

4. **Verify signing (SB still off):**
   ```sh
   bootctl status                # → Secure Boot: disabled (setup)
   sudo sbctl verify             # every UKI + bootloader → "signed"
   ```
   Reboot once on lanzaboote with SB off. If it boots, signing is correct.

### Phase D — enroll keys + enable Secure Boot (firmware dance)

1. Reboot into firmware → Secure Boot → **clear keys / enter Setup Mode**.
2. Back in NixOS:
   ```sh
   sudo sbctl enroll-keys --microsoft
   ```
   `--microsoft` **is not optional here** — it keeps Microsoft's KEK/db so
   third-party **option ROMs still validate**. On an Optimus box the NVIDIA
   GPU's OpROM (and any Windows dual-boot) is signed by MS; enrolling
   custom-keys-only can leave the dGPU or firmware refusing to init. Keep MS keys.
3. Reboot into firmware → **enable Secure Boot** → save.
4. Boot NixOS, confirm:
   ```sh
   bootctl status                # → Secure Boot: enabled (user)
   sudo sbctl verify
   ```

### Phase E — persistence wiring (the impermanence intersection)

Under the ephemeral root, `/etc/secureboot` dies on every boot unless persisted.
Add to the system persist set (`system/modules/fs/persistence.nix` from the
impermanence doc):

```nix
environment.persistence."/persist".directories = [
  # … existing entries …
  "/etc/secureboot"   # lanzaboote/sbctl private signing keys — WITHOUT THIS,
                      # the next nixos-rebuild cannot sign → unsigned/unbootable
];
```

- `neededForBoot` is **not** required for `/etc/secureboot` — the UKIs on the ESP
  are already signed at boot; the keys are only needed at *rebuild/sign* time.
- Sequencing: run `sbctl create-keys` (Phase C) **after** the impermanence
  reinstall, so the keys are written straight onto `/persist` via the bind — or
  create them, then `rsync` `/etc/secureboot` into `/persist/etc/secureboot`
  before the first wipe-reboot.

## Post-enable checklist

- [ ] `bootctl status` → `Secure Boot: enabled (user)`
- [ ] `sudo sbctl verify` → all signed, no "not signed" lines
- [ ] NVIDIA loads: `nvidia-smi` works, Hyprland comes up on the dGPU path
- [ ] Do a throwaway rebuild (`nixos-rebuild boot`) → new gen signs without error
      (proves keys are readable → proves `/etc/secureboot` persisted)
- [ ] Reboot twice → SB stays enabled, keys survive the impermanence wipe
- [ ] If dual-booting: the other-OS entry still boots

## Rollback / recovery

- **Won't boot after enabling SB:** firmware → disable Secure Boot → boot →
  investigate. Enrollment is firmware-side; toggling SB off always recovers.
- **`sbctl verify` shows unsigned after a rebuild:** keys weren't persisted
  (`/etc/secureboot` missing from the persist set) — re-`create-keys`,
  re-enroll. This is the single most likely failure and it's the impermanence link.
- **Bricked-feeling firmware:** "Restore factory keys" / "Reset to Setup Mode"
  in BIOS re-installs the vendor + MS keys.
- Keep a **known-good generation** in the systemd-boot menu; lanzaboote keeps
  signed older gens too.

## Risks / gotchas (one-liners)

- **ESP left at 500M** → UKIs won't fit after 2-3 generations; decide the 2G bump
  during the disko rewrite, not after.
- **Enroll without `--microsoft`** → NVIDIA OpROM / Windows may stop validating.
- **`/etc/secureboot` not persisted** → first post-wipe rebuild produces unsigned
  UKIs → next boot fails SB verification.
- **NVIDIA out-of-tree module** → *not* a problem: NixOS doesn't set
  `module.sig_enforce`, so unsigned kernel modules still load under Secure Boot.
  Only the UKI is signed, and that's enough.
- **`canTouchEfiVariables` stays false** → systemd-boot/lanzaboote can't write the
  NVRAM boot entry; flip it true in Phase B.
- **Doing SB enrollment during the impermanence reinstall** → two failure axes at
  once. Verify the ephemeral root boots unsigned first, then enroll.

## Open questions before executing

- Firmware capability: does the BIOS actually expose Setup Mode / custom key
  enrollment? Confirm in the menu *before* committing (some locked OEM firmwares
  don't). If not → this whole plan is a no-go, stop here.
- Dual-boot: is there another OS on the ESP that needs MS keys to stay bootable?
  (Answers whether `--microsoft` is merely advisable or mandatory.)
- TPM measured boot: worth pairing SB with `systemd-cryptenroll` TPM2 unlock
  later? Only relevant if you add LUKS — current disko is unencrypted, so out of
  scope for this pass.
