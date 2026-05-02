# Impermanence plan

> [!NOTE]
> Planning doc, not yet executed. Goal: rebuild `nyx` with an ephemeral root that
> wipes every boot, keeping only an explicit `/persist` set via
> [`nix-community/impermanence`](https://github.com/nix-community/impermanence).

## TL;DR

- **Mechanism:** btrfs root + rollback to a blank `@` snapshot on every boot
  (the "Erase your darlings" pattern).
- **Why not tmpfs:** 30 GiB RAM, 0 swap, with browsers + Android Studio + emulators
  on the menu — too tight without zram/swap, and the cost of a real disk for
  `/persist` is zero anyway.
- **Why not zfs:** single-disk laptop, kernel-version coupling, no snapshot
  benefit we'd actually use.
- **Cost:** disko config rewrite + reinstall + first-boot fixups. The flake-side
  work is small. The reinstall is the scary bit.

## Current state (snapshot)

- `/dev/nvme0n1p2` vfat 500M → `/boot`
- `/dev/nvme0n1p3` ext4 938G → `/`
- 30 GiB RAM, 0 swap
- Disko already in flake at `system/hosts/nyx/disko-config.nix`
- Home Manager wired in `flake.nix` via `homeModules`

## Target disk layout (btrfs)

```
nvme0n1
├─ p1  EF02   1M     grub MBR
├─ p2  EF00   500M   /boot         (vfat, ESP)
└─ p3  100%          btrfs
       ├─ @         → /         (rolled back to blank on boot)
       ├─ @nix      → /nix      (persisted, noatime, compress=zstd)
       ├─ @persist  → /persist  (persisted, neededForBoot=true)
       ├─ @swap     → /swap     (optional, ~16G, for hibernate)
       └─ @snap     → /.snapshots (optional)
```

`@` gets snapshotted empty once at install (`@-blank`); on every boot the
initrd does `btrfs subvolume delete @ && btrfs subvolume snapshot @-blank @`
before mount.

## Phase plan

1. **Flake plumbing** — no disk changes yet, fully reversible.
   - Add `inputs.impermanence.url = "github:nix-community/impermanence";`
   - Add `inputs.impermanence.nixosModules.impermanence` to `nixosModules` in
     `flake.nix`.
   - Add `inputs.impermanence.homeManagerModules.impermanence` to `homeModules`.
2. **Modules** — new files, not yet imported by hosts:
   - `system/modules/fs/persistence.nix` → `environment.persistence."/persist"`.
   - `system/modules/fs/btrfs-rollback.nix` → initrd unit that wipes `@`.
   - `home/modules/persistence.nix` → `home.persistence."/persist/home/<user>"`.
3. **Disko rewrite** — `system/hosts/nyx/disko-config.nix` becomes the layout
   above. Do NOT enable `disko.enableConfigRebuildHook`-style auto-apply;
   keep it declarative for the next reinstall only.
4. **Migration** — see [Migration procedure](#migration-procedure).
5. **First boot fixups** — see [Post-reboot checklist](#post-reboot-checklist).

## What to persist

### System level → `/persist`

`environment.persistence."/persist".directories` / `.files`:

**Mandatory (the system breaks without these):**

- `/etc/machine-id` *(file)*
- `/etc/ssh/ssh_host_*` *(files)*
- `/var/log`
- `/var/lib/nixos` — UID/GID stable mapping, **do not skip**
- `/var/lib/systemd` — timers, random-seed
- `/var/lib/bluetooth` — paired devices
- `/var/lib/NetworkManager` *(verify whether NM is the stack — check
  `system/modules/core/`)*

**Recommended:**

- `/var/lib/colord` — color profiles
- `/var/lib/upower` — battery history (battery-notifier consumes this)
- `/var/lib/flatpak` — nix-flatpak installs land here
- `/var/lib/iwd` *(only if iwd is the wifi backend)*
- `/var/lib/cups` *(only if printing enabled)*

**Conditional on services you may add:**

- `/var/lib/docker` or `/var/lib/containers`
- `/var/lib/libvirt`
- `/var/lib/tailscale`

> [!IMPORTANT]
> agenix host key (`/etc/ssh/ssh_host_ed25519_key`) MUST live on `/persist`
> and the `/persist` mount MUST have `neededForBoot = true`, otherwise agenix
> can't decrypt secrets at activation time.

### User level → `/persist/home/luisnquin`

`home.persistence."/persist/home/luisnquin".directories`:

**Identity / secrets — non-negotiable:**

- `.ssh`
- `.gnupg`
- `.password-store`
- `.pki`
- `.config/gdrive3`
- `.config/gh` *(if used)*
- `.local/share/keyrings`

**AI agents (the explicit ask):**

- `.claude`, `.claude-monitor` *(also `.claude.json` as a file)*
- `.codex`
- `.gemini`
- `.antigravity`
- `.cursor`, `.config/Cursor`
- `.agents`
- `.config/opencode`, `.local/state/opencode`
- `.config/mcp`
- `.config/encore`

**Browsers — heavy login state:**

- `.config/zen` *(explicit ask)*
- `.config/chromium`
- `.config/Google` *(Chrome)*
- `.mozilla` *(if firefox shows up later)*

**Chat:**

- `.config/discord` *(explicit ask)*
- `.config/Equicord`
- `.config/Vencord` *(if nixcord switches to it)*

**Dev tooling — losing these is real pain:**

- `.config/direnv` *(allowlist; explicit ask)*
- `.local/share/direnv` *(cached envs; debatable but rebuilding 50 hurts)*
- `.local/share/nix`
- `.local/state/nix/profiles` — `~/.nix-profile` resolves through here
- `.local/state/home-manager`
- `.config/git`, `.config/jj`, `.config/lazygit`, `.config/lazydocker`
- `.config/go`, `go` *(GOPATH)*
- `.cargo`, `.rustup`
- `.npm`, `.npm-global`, `.bun-global`
- `.gradle`
- `.android` *(adb keys, AVDs)*
- `.expo`, `.maestro`, `.eas-cli-nodejs`

**Media / IDE state:**

- `.config/obs-studio`
- `.config/spotify`, `.config/spotatui`
- `.config/Antigravity`, `.config/Meltytech`
- `.config/FreeCAD`, `.local/share/FreeCAD`
- `.config/inkscape`

**Desktop:**

- `.config/dconf` — many GTK apps key off this
- `.local/share/applications` — user `.desktop` entries
- `.local/share/icons`, `.icons`
- `.var` — flatpak per-app data
- `.local/share/Steam`, `.steam` *(if gaming on this box)*

**User data — the giant footgun if forgotten:**

- `Documents`, `Pictures`, `Videos`, `Downloads`, `Projects`
- `.face`
- `.zsh` *(history, if stored there)*

### Do NOT persist

The whole point. Let these die every boot:

- `.cache`
- `.local/state/nix` *(except `profiles/`)*
- `.npm/_cacache`, any `node_modules`
- `.cargo/registry/cache`
- `/tmp`, `/var/tmp` (already tmpfs by default on NixOS)

## Migration procedure

> [!CAUTION]
> Disko will recreate the partition table. **Back up `/home/luisnquin` and
> `/etc/nixos` first.** Have a NixOS Live USB ready.

1. **Backup** to an external drive:
   ```sh
   sudo rsync -aAXH --info=progress2 \
     --exclude={'.cache','*/node_modules','.local/share/Trash','go/pkg/mod','.cargo/registry/cache','.npm/_cacache'} \
     /home/luisnquin/ /mnt/backup/luisnquin/
   ```
2. **Export** the agenix age keys and SSH host keys separately — easy to lose.
3. **Boot Live USB**, partition with disko:
   ```sh
   sudo nix --experimental-features 'nix-command flakes' run \
     github:nix-community/disko -- --mode disko \
     /mnt/etc/nixos/system/hosts/nyx/disko-config.nix
   ```
4. **Seed `/persist`** before first activation:
   ```sh
   sudo mkdir -p /mnt/persist/home/luisnquin /mnt/persist/etc/ssh
   sudo cp /mnt/backup/ssh_host_*  /mnt/persist/etc/ssh/
   sudo rsync -aAXH /mnt/backup/luisnquin/ /mnt/persist/home/luisnquin/
   ```
5. **Take the blank snapshot** so the rollback unit has a target:
   ```sh
   sudo btrfs subvolume snapshot -r /mnt/@ /mnt/@-blank
   ```
6. **Install:**
   ```sh
   sudo nixos-install --flake /mnt/etc/nixos#nyx
   ```
7. Reboot.

## Post-reboot checklist

Walk through this in order; each one has caught real users:

- [ ] `systemctl status` is clean — no failed units
- [ ] `id luisnquin` — UID still 1000 (proves `/var/lib/nixos` persisted)
- [ ] `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` matches pre-reinstall
- [ ] `agenix` decryption works (any secret-using service comes up)
- [ ] `bluetoothctl devices` shows old pairings
- [ ] `nmcli c show` shows old wifi profiles with passwords intact
- [ ] Browser logins survive (Zen, Chromium, Google)
- [ ] Discord doesn't ask to log in again
- [ ] gnome-keyring unlocks on login (no "create new keyring" prompt)
- [ ] `direnv allow` list intact (`cd` into a project, no re-allow needed)
- [ ] Reboot a second time, repeat — confirms wipe is actually happening
- [ ] `btrfs subvolume list /` shows `@-blank` still present

## Risks / gotchas (one-liners)

- **Forgetting `/var/lib/nixos`** → UIDs/GIDs drift on every boot, file-ownership chaos.
- **Forgetting `.local/share/keyrings`** → re-type every saved password each boot.
- **Forgetting `.local/state/nix/profiles`** → `~/.nix-profile` symlink dangles.
- **Forgetting `neededForBoot = true` on `/persist`** → agenix fails before persist mounts.
- **Persisting `.cache`** → defeats the purpose; cache files leak across boots.
- **First reboot will surface 5–10 missed paths.** Plan for one fixup cycle.

## Open questions before executing

- Swap/hibernate: do we want a `@swap` subvol with a swapfile, or skip swap entirely?
- Snapshots: enable [`btrbk`](https://github.com/digint/btrbk) for `/persist` rolling backups?
- `/home` strategy: pure impermanence (everything outside the persist list is gone)
  or hybrid (whole `/home` persisted, with `home.persistence` only for documentation)?
  → Recommend **pure**, otherwise impermanence is theatre.
- Migration window: needs a 1–2 hour block where the laptop can be offline.
