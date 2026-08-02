# Flake update

Bump `flake.lock` inputs in `~/.dotfiles`, then answer the three questions a bump
always raises: which overlays are now dead weight, what broke or got renamed
upstream, and what landed that is worth adopting.

## Repo map

- `flake.nix` — every input. Almost all of them `follows` `nixpkgs`, so a nixpkgs
  bump moves the whole closure; a single-input bump is cheap by comparison.
- `flake.toml` — host/user metadata. `nix.stateVersion` and `nix.channel` are
  declarations of intent, not version markers: never bump `stateVersion` just
  because nixpkgs moved.
- `overlays/inputs.nix` — wires flake inputs into `pkgs` (third-party overlays,
  `sickdeck` built from the private git+ssh input, the home-manager news patch).
  Structural, not workarounds — but the fragile parts break on input bumps.
- `overlays/nixpkgs.nix` — the audit target. Mixes temporary workarounds with
  permanent local packages.
- `overlays/patches/` — patch files consumed by `overlays/nixpkgs.nix`.
- `system/` is NixOS, `home/` is home-manager. Host is `nyx`, user alias
  `luisnquin`, single system `x86_64-linux`.

## 0. Preflight

Tree must be clean before the bump so the diff afterwards is only the bump.
A watcher commits `flake.lock` on its own — do not stage or commit that file,
and do not treat its disappearance from `git status` as someone else's change.

## 1. Snapshot before touching anything

Record locked revs and dates; the old timestamp is what scopes every upstream
query later.

```sh
nix flake metadata --json | jq -r '
  .locks.nodes | to_entries[]
  | select(.value.locked.type=="github")
  | "\(.key)\t\(.value.locked.owner)/\(.value.locked.repo)\t\(.value.locked.rev[0:12])\t\(.value.locked.lastModified|todate)"'
```

Record the *upstream* (un-overlaid) version of every package the overlays touch.
This is the before/after signal for the whole audit — an overlay is a candidate
for removal exactly when upstream catches up to what it provides.

```sh
nix eval --raw --impure --expr '
  let f = builtins.getFlake (toString ./.);
      pkgs = import f.inputs.nixpkgs { system = "x86_64-linux"; config = {}; };
  in pkgs.tmux.version'
```

`config = {}` and importing `f.inputs.nixpkgs` directly are the point: they
bypass the overlays, so you read what nixpkgs actually ships. Reading
`.#nixosConfigurations.nyx.pkgs.tmux.version` gives the overlaid value instead.

## 2. Bump

- All inputs: `nix flake update`
- Selected: `nix flake update nixpkgs home-manager`
- A single leaf input: `nix flake update <input>`

Prefer the narrowest bump that serves the reason for updating. Anything with
`inputs.nixpkgs.follows = "nixpkgs"` rides along with a nixpkgs bump whether you
asked for it or not.

## 3. Overlay audit

Re-read `overlays/nixpkgs.nix` — it is the source of truth and it drifts. Sort
each entry into one of two classes:

**Temporary** — a pin, a patch, or a workaround for an upstream defect. These
carry a comment naming the exit condition ("drop once nixpkgs ships 3.8",
"Fixed upstream in nixpkgs <sha>; drop once nixos-unstable includes it",
"Temporary pin past upstream #510"). Each one is a question to answer on every
bump.

**Permanent** — packages nixpkgs does not have (local Rust/Go builds, plugins)
or deliberate behaviour changes (renaming a binary, injecting a banner, adding
hooks). These only go away if upstream starts shipping the thing.

### Answering a temporary overlay

*Fix identified by commit* — ask GitHub whether the new lock contains it:

```sh
gh api repos/NixOS/nixpkgs/compare/<fix-sha>...<new-locked-rev> --jq '{status, ahead_by, behind_by}'
```

`ahead` or `identical` → the fix is in, drop the overlay. `behind` → the fix
landed after the locked rev, keep it. `diverged` → different branch, check the
merge base before concluding anything.

*Fix identified by version* — compare the upstream eval from step 1 against the
version the overlay pins. Upstream at or past the pin means the override is now
a downgrade, not a fix.

*Fix identified by nothing* — read the package's recent history and look for the
change the comment describes:

```sh
gh api 'repos/NixOS/nixpkgs/commits?path=pkgs/by-name/tm/tmux/package.nix&per_page=20' \
  --jq '.[] | "\(.sha[0:8]) \(.commit.committer.date) \(.commit.message|split("\n")[0])"'
```

*Patch overlays* — a patch that no longer applies fails the build loudly, which
is the answer. A patch that still applies may be redundant: read the upstream
source at the new rev and check whether the fix is already there. Same for the
`grep`/`sed` guards in the hyprland overlay and the `--replace-fail` in the
home-manager overlay in `overlays/inputs.nix` — those fail hard by design when
upstream moves the text they anchor on, so a failure there is information, not a
mystery.

### Answering a permanent overlay

Only one question: does nixpkgs ship it now?

```sh
nix eval --impure --expr '
  let f = builtins.getFlake (toString ./.);
      pkgs = import f.inputs.nixpkgs { system = "x86_64-linux"; config = {}; };
  in builtins.map (n: { name = n; upstream = pkgs ? ${n}; })
     [ "spiceedit" "herdr-sesh" "herdr-pluck" ]'
```

If upstream has it, compare version and build inputs before switching — a
nixpkgs package with an older version or a missing `postInstall` is not a
replacement. The same question applies to flake inputs that exist only to
deliver one package.

Report every temporary overlay you checked and the verdict, including the ones
that stay. "Still needed" is a result; silence is not.

## 4. Option drift

Renamed, deprecated and removed options surface as eval warnings, so evaluate
both configurations and read the warnings rather than skipping to the build:

```sh
nixos-rebuild dry-build --flake .#nyx 2>&1 | rg -i 'warning|deprecat|renamed|removed'
home-manager build --flake . 2>&1 | rg -i 'warning|deprecat|renamed|removed'
```

Treat every rename warning as work: the old name keeps functioning until it does
not, and the warning is the only notice.

Removed *packages* are the sharper edge: nixpkgs replaces the attribute with a
`throw`, so evaluation dies on the first one instead of warning, and the message
names only that package. Fix, re-evaluate, repeat — there may be more behind it.
Removals arrive in sweeps (one dependency gets dropped and everything depending
on it goes with it), so read the batch rather than the single package:

```sh
gh api "repos/NixOS/nixpkgs/commits?path=pkgs/top-level/aliases.nix&sha=<new-rev>&per_page=100" \
  --jq '.[] | "\(.commit.committer.date[0:10]) \(.commit.message|split("\n")[0])"'
```

Watch for `Revert "<pkg>: remove"` in that log — some removals get walked back
within days, and a reverted one needs no work at all.

Two ways out, in order of preference: switch to a surviving package, or, when
that would change behaviour the user chose deliberately (a theme, a font, a
fork), resurrect the derivation in `overlays/nixpkgs.nix`. Pull the original
from the commit before the removal and strip what died with it:

```sh
rm=$(gh api "repos/NixOS/nixpkgs/commits?path=<pkg-path>&sha=<new-rev>&per_page=1" --jq '.[0].sha')
parent=$(gh api repos/NixOS/nixpkgs/commits/$rm --jq '.parents[0].sha')
gh api "repos/NixOS/nixpkgs/contents/<pkg-path>?ref=$parent" --jq '.content' | base64 -d
```

Resurrecting is a judgement call about *appearance or behaviour the user picked*,
so preserve it exactly — verify the rebuilt output still contains the specific
variant, binary or file the config names, rather than assuming the build passing
means the config is satisfied.

Home-manager's news is patched out of the binary in `overlays/inputs.nix`
(`presentNews` is replaced with `:`), so the usual "N news items" prompt never
appears. Recover that feed from the repo instead — it is the curated list of
what changed and what needs attention:

```sh
gh api repos/nix-community/home-manager/compare/<old-hm-rev>...<new-hm-rev> \
  --jq '.files[].filename' | rg 'modules/misc/news'
```

Then read the added entries. The compare API caps at 300 files; on a large bump,
list `modules/misc/news/<year>/` and read entries dated after the old lock.

## 5. What landed that is worth taking

Scope this to what the configuration already uses — a full nixpkgs changelog is
noise. Three sources, in order of value:

1. **Module changes in areas this config enables.** Derive the areas from
   `system/modules/` and `home/modules/`, then query each path between the old
   and new lock dates:

   ```sh
   gh api 'repos/NixOS/nixpkgs/commits?sha=<new-rev>&path=nixos/modules/services/<area>&since=<old-lock-ISO>' \
     --jq '.[] | "\(.sha[0:8]) \(.commit.message|split("\n")[0])"'
   ```

   New options usually arrive as "nixos/<module>: add <option>" commits.

2. **Release notes at the new rev** — `nixos/doc/manual/release-notes/` in
   nixpkgs, for the incompatibility list more than the feature list.

3. **Overlay and input retirement**, from step 3. Deleting an overlay is worth
   more than adopting an option.

Propose, do not apply. New options are a decision, not a consequence of the
bump; land the bump first and raise adoptions separately.

## 6. Verify

```sh
nix flake check
nixos-rebuild dry-build --flake .#nyx
home-manager build --flake .
```

Compare what actually changed in the closure rather than guessing from the lock
diff — `nvd` names every package that moved:

```sh
nvd diff /run/current-system ./result
```

If any `.nix` file was edited during the audit: `alejandra .`, plus `deadnix`
and `statix check` — CI runs the first two.

## 7. Reporting

State, in this order: which inputs moved and by how much; which overlays were
checked and their verdicts (dropped, kept with reason, newly needed); which
options were renamed or removed and what was done about them; what is newly
available and worth a separate change; and what failed to build.

Commit code changes separately from the lock — the lock is the watcher's. Follow
the repo convention: conventional commits, lowercase titles, concrete code
changes in the message rather than motivation.
