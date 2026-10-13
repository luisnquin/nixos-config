---
name: nix-patch
description: Change upstream software in this dotfiles repo with an intent-named .patch under overlays/patches/<pkg>, wired through overlays/nixpkgs.nix, made from a temporary ./<name> clone. Use when asked to patch a package, change a program's behavior, or follow the overlays/patches pattern; never a wrapper or vendored package.
---

# Nix patch

Change upstream software in this repo with a `.patch` file applied through an
overlay. Not a wrapper, not a vendored package, not a new pattern in `flake.nix`.

## Where things go

- Patch: `overlays/patches/<package>/<intent>.patch`. The file name states what
  the patch makes the program do (`show-worktree-path.patch`,
  `default-no-daemon.patch`), never a ticket, a date or `fix.patch`.
- Wiring: `overlays/nixpkgs.nix`, appended to `patches` in an `overrideAttrs`
  block for that package. Reuse the package's existing block if there is one.
- Global scope: the overlay applies everywhere. darwin-config consumes these
  overlays, so never duplicate a patch there.

## Steps

1. A clone at `./<name>` in the repo root is a temporary reference copy. Never
   commit it, package it or wire it in.
2. Check out the exact rev nix builds: read it from `flake.lock` or from the
   package's `src`. A patch made against another rev fails at `patchPhase`.
3. Edit the clone, then `git -C ./<name> diff > overlays/patches/<pkg>/<intent>.patch`.
   One intent per patch. Split a change that does two things into two patches.
4. Add the path to the package's `patches` list.
5. When updating an existing patch, regenerate it from the clone. Never
   hand-edit the hunks.
6. Do not add tests, and do not build unless asked. If the user allows a check,
   `nix build .#<attr>` on that package only, never a full switch.
7. Commit with the scope used by sibling patches: `git log --oneline -- overlays/patches/<pkg>`.
   Name a concern-level scope when one exists, e.g. `feat(agents/codex)`.
   Otherwise use `feat(overlays/<pkg>)`.
8. Delete the clone, or tell the user it is still there.

## Not this skill

Bumping a version, fixing a hash or disabling a check belongs in an overlay
attribute override, without a patch file.
