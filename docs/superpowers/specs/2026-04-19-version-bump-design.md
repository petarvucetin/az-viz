# Auto Version Bump on Every Commit — Design Spec

**Date:** 2026-04-19
**Status:** Design approved; implementation to follow.

## 1. Problem

`az-plotter`'s version lives in three files that must stay in sync:
- `Cargo.toml` — workspace `version` field
- `src-tauri/tauri.conf.json` — `package.version` (used for installer filenames and MSI upgrade logic)
- `ui/package.json` — `version`

The user wants every commit to advance the version, so that every installer artifact is traceable to a specific commit without manual bump-and-sync work.

## 2. Decision

Every `git commit` patch-bumps the version across all three files automatically, via a pre-commit Git hook. One commit = one version increment.

| Case | Behavior |
|---|---|
| Normal commit | Auto-bump patch (e.g. `0.1.0 → 0.1.1`), stage all three files into the commit. |
| Manual version edit | If the staged `Cargo.toml` has a version different from `HEAD:Cargo.toml`, respect the manual choice and just sync the other two files to match. |
| `--amend --no-edit` (no new content) | Skip the bump (staged diff vs HEAD is empty). |
| `--amend` with new content | Treat as a new change; bump. |
| `git commit --no-verify` | Skip the hook entirely. Escape hatch. |

Bump direction is **patch-only**. Minor/major bumps are a manual act — edit `Cargo.toml`, commit, the hook's sync logic picks it up.

## 3. Files

| File | Purpose |
|---|---|
| `scripts/bump-version.js` | Node script with the bump/sync logic. No dependencies. |
| `scripts/hooks/pre-commit` | One-line shell wrapper invoking the Node script. |

Hooks live under version control in `scripts/hooks/` rather than `.git/hooks/` so they survive `git clone` and are reviewable in diffs.

## 4. One-time setup

```
git config core.hooksPath scripts/hooks
```

Sets Git to look for hooks in the version-controlled directory. Local config; must be run once per clone.

## 5. Out of scope

- UI display of the version number (deferred — separate discussion).
- Minor/major bump via conventional-commits — hook respects manual edits; automation not added.
- Git tag creation per commit.
- Changelog / release-note generation.
- CI integration (no CI exists).

## 6. Success criteria

- Committing any change bumps the patch version in all three files, in the same commit.
- Manually editing `Cargo.toml` to a different version and committing results in the new version being synced into the other two files (not further bumped).
- `git commit --amend --no-edit` does not change the version.
- `git commit --no-verify` does not change the version.
