# Icon Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to execute this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a production-ready Windows `.ico` file containing the az-plotter icon at all required scales (16×16 through 256×256 px), replace the placeholder, and verify it appears in the installer.

**Architecture:** Build in three phases: (1) SVG master with exact geometry from spec, (2) rasterize to PNG at 5 standard sizes using a tool (Inkscape or ImageMagick), (3) composite into `.ico` using a Windows icon tool or ImageMagick.

**Tech Stack:** SVG (hand-written), Inkscape CLI or ImageMagick (`convert`), `magick` or `icoutils` for ICO composition.

---

## Phase 1: Create SVG Master

### Task 1: Write SVG master file with exact geometry

**Files:**
- Create: `src-tauri/icons/az-plotter-icon.svg`

- [ ] **Step 1: Create SVG file with viewBox and gradient definition**

Create `src-tauri/icons/az-plotter-icon.svg`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<svg width="128" height="128" viewBox="0 0 128 128" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bgGradient" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#f97316"/>
    </linearGradient>
  </defs>

  <!-- Background with rounded corners -->
  <rect x="2" y="2" width="124" height="124" rx="22" fill="url(#bgGradient)"/>

  <!-- Concentric rings (4 white circles at 45% opacity) -->
  <circle cx="64" cy="64" r="50" fill="none" stroke="#ffffff" stroke-width="1.5" opacity="0.45"/>
  <circle cx="64" cy="64" r="42" fill="none" stroke="#ffffff" stroke-width="1.5" opacity="0.45"/>
  <circle cx="64" cy="64" r="34" fill="none" stroke="#ffffff" stroke-width="1.5" opacity="0.45"/>
  <circle cx="64" cy="64" r="26" fill="none" stroke="#ffffff" stroke-width="1.5" opacity="0.45"/>

  <!-- Spokes (6 navy lines from hub to outer nodes) -->
  <g stroke="#0b2447" stroke-width="3" stroke-linecap="round" fill="none">
    <line x1="64" y1="64" x2="64" y2="18"/>   <!-- Top: 0° -->
    <line x1="64" y1="64" x2="104" y2="42"/>  <!-- Top-right: 60° -->
    <line x1="64" y1="64" x2="104" y2="86"/>  <!-- Bottom-right: 120° -->
    <line x1="64" y1="64" x2="64" y2="110"/>  <!-- Bottom: 180° -->
    <line x1="64" y1="64" x2="24" y2="86"/>   <!-- Bottom-left: 240° -->
    <line x1="64" y1="64" x2="24" y2="42"/>   <!-- Top-left: 300° -->
  </g>

  <!-- Outer nodes (6 colored circles with navy stroke) -->
  <circle cx="64" cy="18" r="9" fill="#00b4d8" stroke="#0b2447" stroke-width="2"/>  <!-- Cyan -->
  <circle cx="104" cy="42" r="9" fill="#0077b6" stroke="#0b2447" stroke-width="2"/> <!-- Royal -->
  <circle cx="104" cy="86" r="9" fill="#9d4edd" stroke="#0b2447" stroke-width="2"/> <!-- Violet -->
  <circle cx="64" cy="110" r="9" fill="#2d6a4f" stroke="#0b2447" stroke-width="2"/> <!-- Forest -->
  <circle cx="24" cy="86" r="9" fill="#c9184a" stroke="#0b2447" stroke-width="2"/>  <!-- Crimson -->
  <circle cx="24" cy="42" r="9" fill="#ffd60a" stroke="#0b2447" stroke-width="2"/>  <!-- Gold -->

  <!-- Central hub (white with navy stroke) -->
  <circle cx="64" cy="64" r="15" fill="#ffffff" stroke="#0b2447" stroke-width="3"/>
</svg>
```

- [ ] **Step 2: Validate SVG opens in a browser**

Open `src-tauri/icons/az-plotter-icon.svg` in a web browser (Firefox, Chrome, Safari, or Edge). Verify:
- Orange gradient runs diagonally from TL (white) to BR (orange)
- 6 colored nodes visible at cardinal + 60° angles
- Central white hub with navy stroke is centered
- 4 white concentric rings are visible but subtle

Expected: Icon visually matches the design spec and approved concept.

- [ ] **Step 3: Commit SVG master**

```bash
git add src-tauri/icons/az-plotter-icon.svg
git commit -m "assets: add SVG master for az-plotter icon"
```

---

## Phase 2: Rasterize SVG to PNG

### Task 2: Rasterize SVG to 5 PNG sizes

**Files:**
- Create: `src-tauri/icons/icon-16.png`, `icon-32.png`, `icon-64.png`, `icon-128.png`, `icon-256.png`
- Source: `src-tauri/icons/az-plotter-icon.svg`

**Tool options:**
- **Inkscape CLI** (Windows: installed via scoop/chocolatey or `winget install inkscape`)
- **ImageMagick** (`magick` / `convert` command; Windows: `winget install imagemagick`)

**Approach:** Use ImageMagick's `magick` command (simpler one-liner per size, widely available on Windows).

- [ ] **Step 1: Install ImageMagick if not present**

Check if ImageMagick is installed:
```bash
magick --version
```

If not found, install via Windows Package Manager:
```bash
winget install imagemagick
```

Then verify:
```bash
magick --version
```

Expected: `Version: ImageMagick 7.x.x-Q16` or similar.

- [ ] **Step 2: Rasterize SVG to 16×16 PNG**

From the project root:
```bash
magick -background none -density 300 src-tauri/icons/az-plotter-icon.svg -resize 16x16 -flatten src-tauri/icons/icon-16.png
```

Verify the file was created:
```bash
ls -la src-tauri/icons/icon-16.png
```

Expected: File exists, ~5–10 KB.

- [ ] **Step 3: Rasterize SVG to 32×32 PNG**

```bash
magick -background none -density 300 src-tauri/icons/az-plotter-icon.svg -resize 32x32 -flatten src-tauri/icons/icon-32.png
```

Verify:
```bash
ls -la src-tauri/icons/icon-32.png
```

Expected: File exists, ~10–15 KB.

- [ ] **Step 4: Rasterize SVG to 64×64 PNG**

```bash
magick -background none -density 300 src-tauri/icons/az-plotter-icon.svg -resize 64x64 -flatten src-tauri/icons/icon-64.png
```

Verify:
```bash
ls -la src-tauri/icons/icon-64.png
```

Expected: File exists, ~15–25 KB.

- [ ] **Step 5: Rasterize SVG to 128×128 PNG**

```bash
magick -background none -density 300 src-tauri/icons/az-plotter-icon.svg -resize 128x128 -flatten src-tauri/icons/icon-128.png
```

Verify:
```bash
ls -la src-tauri/icons/icon-128.png
```

Expected: File exists, ~25–40 KB.

- [ ] **Step 6: Rasterize SVG to 256×256 PNG**

```bash
magick -background none -density 300 src-tauri/icons/az-plotter-icon.svg -resize 256x256 -flatten src-tauri/icons/icon-256.png
```

Verify:
```bash
ls -la src-tauri/icons/icon-256.png
```

Expected: File exists, ~40–60 KB.

- [ ] **Step 7: Verify all PNGs exist**

```bash
ls -la src-tauri/icons/icon-*.png
```

Expected output (5 files):
```
-rw-r--r-- ... icon-16.png
-rw-r--r-- ... icon-32.png
-rw-r--r-- ... icon-64.png
-rw-r--r-- ... icon-128.png
-rw-r--r-- ... icon-256.png
```

- [ ] **Step 8: Commit rasterized PNGs**

```bash
git add src-tauri/icons/icon-{16,32,64,128,256}.png
git commit -m "assets: rasterize icon to PNG at 5 standard sizes"
```

---

## Phase 3: Bundle into Windows ICO

### Task 3: Create Windows `.ico` file from PNGs

**Files:**
- Create: `src-tauri/icons/icon.ico` (replaces existing placeholder)
- Source: `src-tauri/icons/icon-{16,32,64,128,256}.png`

**Tool:** ImageMagick `magick convert` (can bundle multiple PNGs into a single ICO).

- [ ] **Step 1: Create `.ico` from all 5 PNG sizes**

From the project root:
```bash
magick src-tauri/icons/icon-256.png src-tauri/icons/icon-128.png src-tauri/icons/icon-64.png src-tauri/icons/icon-32.png src-tauri/icons/icon-16.png -alpha off -colors 256 src-tauri/icons/icon.ico
```

Verify the file was created:
```bash
ls -lh src-tauri/icons/icon.ico
file src-tauri/icons/icon.ico
```

Expected output:
```
-rw-r--r-- ... icon.ico (should be ~100–150 KB, smaller than the sum of PNGs due to ICO compression)
icon.ico: MS Windows icon resource - X icons, 256x256, 32 bits/pixel
```

- [ ] **Step 2: Commit the new `.ico` file**

```bash
git add src-tauri/icons/icon.ico
git commit -m "assets: replace icon.ico with new 6-spoke design"
```

---

## Phase 4: Integration & Verification

### Task 4: Rebuild Windows installer and verify icon appearance

**Files:**
- Modified (config, no code): `src-tauri/tauri.conf.json` (no change needed; already points to `icons/icon.ico`)

- [ ] **Step 1: Clean build artifacts**

```bash
cargo clean --release
```

This ensures Tauri's bundler re-reads the icon file during the next build.

- [ ] **Step 2: Rebuild installer**

From the project root:
```bash
cargo tauri build 2>&1 | tee build.log
```

This will:
1. Run `npm --prefix ui run build` (rebuild UI)
2. Compile Rust release binary
3. Bundle with Tauri: pick up `src-tauri/icons/icon.ico` and embed into executable + installer

Expected output (tail):
```
Finished 2 bundles at:
  D:\AI\projects\az-plotter\target\release\bundle\msi\az-plotter_0.1.0_x64_en-US.msi
  D:\AI\projects\az-plotter\target\release\bundle\nsis\az-plotter_0.1.0_x64-setup.exe
```

- [ ] **Step 3: Inspect Windows icon in MSI**

The MSI and NSIS installers both embed and display the icon. Extract and visually inspect:

**Option A (Windows Explorer):** Right-click the `.msi` or `.exe`, choose "Properties". Under "Details" tab, verify the icon thumbnail shows the new 6-spoke design with orange gradient.

**Option B (Extract via orca tool):** If you have ORCA (Windows Installer editor), open the `.msi`, navigate to the Icon table, and verify the icon resource is embedded correctly. (Optional; visual inspection via Explorer is usually sufficient.)

Expected: Icon thumbnail in file properties shows the new design (orange gradient, 6 colored nodes, white hub visible).

- [ ] **Step 4: Verify installer icon at 32×32 size**

Navigate to `target/release/bundle/nsis/` in Windows Explorer, switch to "List" or "Details" view, and right-click the `.exe`:
- **Properties → Details:** Icon thumbnail should be crisp and readable at 32×32 (typical size displayed here)
- The 6 nodes should be distinguishable; the orange gradient should be visible; the white hub should stand out from the orange background

Expected: Icon is crisp, colors are accurate, no artifacts or blurriness.

- [ ] **Step 5: (Optional) Test installer UI**

Double-click `az-plotter_0.1.0_x64-setup.exe` to launch the NSIS installer. Verify:
- The installer wizard window displays the app icon in the top-left corner of the window
- Icon is clear and matches the design (orange gradient, 6 nodes)
- No pixelation or color degradation

If all looks good, cancel the installer.

Expected: Icon displays correctly in the installer wizard window.

- [ ] **Step 6: Final verification commit**

No code changes, but document the verification:
```bash
git log --oneline | head -5
```

Verify the last two commits are:
```
commit_hash_2 assets: replace icon.ico with new 6-spoke design
commit_hash_1 assets: rasterize icon to PNG at 5 standard sizes
```

If verification is complete and icon looks correct, add a note to the commit message or a follow-up comment:
```bash
git log -1 --format=%B
```

Expected: The most recent commit message is "assets: replace icon.ico with new 6-spoke design".

---

## Spec Coverage Check

| Spec Section | Implementation Task |
|---|---|
| 2. Visual Concept (geometry) | Task 1: SVG master with exact coordinates |
| 3. Color Palette | Task 1: SVG colors from spec table |
| 4. Geometry & Measurements | Task 1: 128px canvas, hub r=15, nodes r=9 at 38px radius, rings at r=26/34/42/50, corner radius 22px |
| 5. Design Intent (scale readability) | Task 2: Rasterize to all 5 sizes (16–256 px) for visual inspection at each scale |
| 6. Asset Output Requirements (SVG, PNG, ICO) | Task 1 (SVG), Task 2 (PNG), Task 3 (ICO) |
| 7. Technical Notes (SVG, anti-alias, gradient) | Task 2: ImageMagick rasterization inherently anti-aliases; gradient defined in SVG |
| 8. Size & Deployment (32×32 in installer) | Task 4: Rebuild and verify in NSIS/MSI at 32×32 size |
| 9. Success Criteria (readable at 32×32, no config changes) | Task 4: Visual verification; tauri.conf.json unchanged |

---

## Execution Options

Plan complete and saved to `docs/superpowers/plans/2026-04-19-icon-implementation.md`.

Two ways to execute:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task (SVG → PNG → ICO → verify), review after each, fast iteration.

**2. Inline Execution** — Execute all tasks in this session with checkpoints for visual inspection between phases.

Which approach?
