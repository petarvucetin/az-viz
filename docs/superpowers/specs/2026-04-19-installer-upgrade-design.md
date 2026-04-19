# Installer Upgrade Behavior — Design Spec

**Date:** 2026-04-19
**Status:** Design approved; implementation to follow.

## 1. Problem

The Windows installers (MSI and NSIS) are produced by Tauri with no explicit upgrade configuration. Tauri's defaults do already perform silent in-place upgrades when a previous install of the same `identifier` is detected, but the defaults are implicit — they could shift in a future Tauri CLI release and they are not self-documenting for future maintainers. We want the upgrade contract to be explicit and pinned in `tauri.conf.json`.

## 2. Goal

When a user runs a newer `az-plotter` installer on a machine that already has an older `az-plotter` installed:

- The old version is uninstalled **silently** (no prompt, no manual Control Panel step).
- The new version is installed in its place.
- User data under `%APPDATA%\com.station5solutions.az-plotter\` is **preserved** across the upgrade.

The behavior must be pinned in `tauri.conf.json` rather than relying on Tauri CLI defaults.

## 3. Non-goals

- Configurable upgrade prompts (we explicitly chose silent; no "Continue?" dialog).
- Cross-user upgrades on shared machines (we use `currentUser` mode, so each OS user has their own install).
- User data migration / reset (user data is untouched; a future spec can address reset if ever needed).
- Admin-elevated installs (ruled out by choosing `currentUser` over `perMachine`).
- Version bumping mechanics (covered by a separate, pending discussion).

## 4. Decisions (locked)

| Dimension | Decision |
|---|---|
| Upgrade prompt | Silent (no user interaction). |
| Install scope | Per-user (`currentUser`), no admin elevation. |
| User data on upgrade | Preserved (lives outside the install directory). |
| MSI locale | Pinned to `en-US`. |
| Upgrade detection | NSIS: `HKCU\Software\<identifier>` registry key. MSI: `UpgradeCode` GUID auto-derived from `identifier`. |

## 5. Configuration Change

One addition to `src-tauri/tauri.conf.json`, inside `tauri.bundle`:

```json
"windows": {
  "nsis": {
    "installMode": "currentUser"
  },
  "wix": {
    "language": ["en-US"]
  }
}
```

No changes required to:
- `tauri.windows` (window configuration, distinct from `tauri.bundle.windows`).
- `identifier` (must remain stable — it is the upgrade-detection key for both installers).
- Cargo workspace version.
- Icon assets.

## 6. Why this works

**NSIS upgrade path:**
1. Tauri's NSIS template writes install metadata under `HKCU\Software\com.station5solutions.az-plotter` at install time.
2. When a new installer runs, its template script checks that key.
3. If present, the old installer's `uninstall.exe` is invoked silently (`/S` mode) to remove the prior binaries.
4. The new installer then proceeds to install its own files.
5. A consistent `installMode` across releases is what makes step 2 reliable — if a previous install was `currentUser` and a new one is `perMachine`, the registry check misses it and side-by-side installs can occur. Pinning `installMode` eliminates that class of bug.

**MSI upgrade path:**
1. Tauri's WiX template derives a stable `UpgradeCode` GUID deterministically from `identifier`.
2. The WiX template includes a `<MajorUpgrade Schedule="afterInstallInitialize" />` element.
3. When a new MSI is run and finds an install with the same `UpgradeCode` but different `ProductCode`, the Windows Installer service uninstalls the old product before installing the new one.
4. Pinning the locale (`en-US`) ensures the filename, resource strings, and upgrade table stay consistent release-to-release.

**User data preservation:**
- Neither installer touches `%APPDATA%\com.station5solutions.az-plotter\` — this directory is created and managed by the running app, not the installer.
- `.azp` session files saved by users to arbitrary paths are also untouched.

## 7. Verification

- **Build smoke test:** `cargo tauri build` succeeds and produces `az-plotter_0.1.0_x64_en-US.msi` and `az-plotter_0.1.0_x64-setup.exe`. Filename format unchanged.
- **Upgrade path (manual, deferred):** requires two different-version installers. Proposed test when a 0.1.1 build exists: install 0.1.0, create a sample project, install 0.1.1, verify (a) no side-by-side entry in "Apps & features", (b) sample project still loads.

The upgrade-path test is out of scope for the immediate implementation (we only have 0.1.0); it is documented here so it can be executed at the next version bump.

## 8. Success Criteria

- `tauri.conf.json` contains the explicit `windows.nsis.installMode` and `windows.wix.language` fields.
- Build produces identical filenames as before.
- Design is documented and committed.

## 9. Next Steps

1. Apply the `tauri.conf.json` change.
2. Rebuild installers; confirm filenames and sizes are unchanged (± rounding).
3. Commit config + design spec.
