---
name: nix-triage
description: Diagnose a pasted nixos-rebuild, home-manager switch or nix build failure or eval warning. Name the failing derivation and what pulls it in, check upstream, then bump, patch, override or comment out, without running the build. Use when the user pastes a Nix error or asks which package breaks the build.
---

# Nix triage

The user pastes a failing `nixos-rebuild`, `home-manager switch` or `nix build`
output, or an eval warning. Find the cause and fix it without running the build:
the user rebuilds with `nyx update`, and a full build can take 30 minutes.

## Steps

1. Read the log from the bottom up. Name the first derivation that failed, not
   the dependents listed after it.
2. Find which config path pulls it in:
   - `nix why-depends` on a store path from the log;
   - `nix-store -q --referrers` on the failed `.drv`;
   - grep the repo for the package name.
   Report the chain: `pdal <- gdal <- qgis <- home/modules/...`.
3. Check whether upstream already fixed it:
   - read the locked rev in `flake.lock`;
   - compare against the input's newer commits, open issues and PRs (`gh`).
   If a fix landed after the locked rev, the bump is the fix.
4. Pick one fix, in this order of preference:
   - **Bump** the single input that carries the fix (`nix flake update <input>`).
   - **Patch** the source when the failure is a small code issue. Hand off to the `nix-patch` skill.
   - **Override** for hash mismatches, flaky tests (`doCheck = false`) or a missing flag. Put it in `overlays/nixpkgs.nix`.
   - **Comment out** the package with `# until <upstream issue/PR> is fixed`. Only when nothing is used from it.
5. Eval warnings (`stdenv.isLinux is deprecated`, renamed options): find the
   source with `--show-trace`. Fix it in this repo, or say which input owns it.
   Never patch an input's code to silence a warning.
6. Say which fix you picked and why, then stop. Do not run the rebuild unless
   asked.

## Rules

- Fix only the reported failure. A second failure behind it is a new report.
- `nixgrep <name>` finds store paths. Never `ls /nix/store | grep`.
- Darwin failures are evaluated and built on rose, not here.
