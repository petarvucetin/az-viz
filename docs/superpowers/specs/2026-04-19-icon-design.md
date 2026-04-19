# az-plotter Icon Design Spec

**Date:** 2026-04-19  
**Status:** Design approved; ready for implementation (asset generation and integration).

## 1. Overview

A custom desktop application icon for `az-plotter` — an Azure CLI network dependency visualizer. The icon should read clearly at multiple scales (16×16 through 256×256 px) and convey "network topology" instantly to Azure operators.

## 2. Visual Concept

**Motif:** 6-spoke radial hub with 4 nested concentric rings.

- **Central hub:** white circle with navy stroke
- **Spokes:** 6 navy lines radiating outward (0°, 60°, 120°, 180°, 240°, 300°)
- **Outer nodes:** 6 colored circles (one per spoke) with navy stroke; each a distinct saturated color
- **Connecting rings:** 4 concentric white circles at 45% opacity, creating expanding network topology layers
- **Background:** Diagonal linear gradient from white (top-left) to orange (bottom-right)
- **Container:** 22px corner radius (rounded square silhouette for modern app-icon aesthetics)

## 3. Color Palette

| Element | Color | Usage |
|---------|-------|-------|
| **Gradient start (TL)** | `#ffffff` | Top-left corner of background |
| **Gradient end (BR)** | `#f97316` | Bottom-right corner of background |
| **Spokes & strokes** | `#0b2447` | Navy; all line work and outer node strokes |
| **Hub fill** | `#ffffff` | White interior of central circle |
| **Outer node 1** | `#00b4d8` | Cyan |
| **Outer node 2** | `#0077b6` | Royal blue |
| **Outer node 3** | `#9d4edd` | Violet |
| **Outer node 4** | `#2d6a4f` | Forest green |
| **Outer node 5** | `#c9184a` | Crimson |
| **Outer node 6** | `#ffd60a` | Gold |
| **Rings (opacity)** | `#ffffff @ 0.45` | White concentric circles at 45% transparency |

## 4. Geometry & Measurements

**Canvas:** 128×128 px (scaled to 256×256 for visual design; final SVG master is 128 logical units)

### Hub
- Center: `(64, 64)`
- Radius: 15 px
- Stroke: 3 px navy

### Spokes
- 6 lines, evenly spaced (60° apart)
- Start from hub center `(64, 64)`
- End radius: ~46 px from center (outer nodes sit at 38 px, leaving space for node radius)
- Stroke: 3 px navy, round linecap

### Outer Nodes
- Position radius: 38 px from center
- Radius: 9 px each
- Stroke: 2 px navy
- Angular distribution:
  - Top: 0° → `(64, 18)`
  - Top-right: 60° → `(104, 42)`
  - Bottom-right: 120° → `(104, 86)`
  - Bottom: 180° → `(64, 110)`
  - Bottom-left: 240° → `(24, 86)`
  - Top-left: 300° → `(24, 42)`

### Concentric Rings
- 4 white circles at 45% opacity, no fill
- Stroke: 1.5 px white
- Radii: 26, 34, 42, 50 px (center-to-center)

### Container
- 128×128 canvas
- Corner radius: 22 px (creates rounded-square silhouette)
- Background: linear gradient white → orange (45° angle)

## 5. Design Intent

**Icon readability at scale:**
- **256×256** (design view): All 6 nodes, rings, and gradient are clearly distinguishable
- **128×128** (installer / app): Rings visible but subtle; nodes and color palette readable
- **64×64** (common taskbar/shelf): 6 nodes compress but remain legible; rings compress to visual noise floor (acceptable as supporting detail)
- **32×32** (Windows icon cache): Hub, spokes, and nodes are still clear; rings fade into background texture (expected at this scale)
- **16×16** (small UI, browser favicon): Hub + spoke silhouette dominates; node colors are hints rather than discrete shapes (acceptable; icon is already very compact)

**Color theory:**
- Saturated 6-color palette on white/navy strokes reads distinctly even when nodes compress
- Orange background (complementary to the cool blues/greens of the nodes) creates visual pop and brand distinctiveness
- White gradient start on the light end ensures icon remains visible against light backgrounds and app-list selections

**Narrative:**
- Hub-and-spoke evokes network routing and the "central topology planning" nature of the app
- Concentric rings suggest expanding scope, layers of network topology, and the outward propagation of a graph plan
- Saturated colors (blue/green/purple/etc.) rather than monochrome convey "this is for engineers" and "resource diversity"

## 6. Asset Output Requirements

1. **SVG master file** (`az-plotter-icon.svg`)
   - 128×128 logical viewBox
   - All colors as RGB hex (no gradients in rasterize step, only in SVG)
   - Exported at 1× scale for accuracy

2. **Rasterized PNG files** (lossless, no compression artifacts)
   - 16×16 px
   - 32×32 px
   - 64×64 px
   - 128×128 px
   - 256×256 px
   - Each with anti-aliasing enabled (standard rasterizer setting)

3. **Windows `.ico` file** (`icon.ico`)
   - Composite of 16×16, 32×32, 64×64, 128×128 PNGs
   - 32-bit RGBA (supports transparency, though background is solid)
   - Replaces existing `src-tauri/icons/icon.ico`

4. **Alternate formats** (for future use)
   - PNG 256×256 (macOS app bundle, may be needed for cross-platform later)

## 7. Technical Notes

- **SVG rendering:** No filters, no complex path operations; keep it simple for rasterizer stability across tools
- **Stroke behavior:** Strokes should center on the path geometry (SVG default), no offset
- **Gradient direction:** Linear, 45° angle (TL to BR); starts at `(0, 0)` white, ends at `(128, 128)` orange
- **Anti-aliasing:** Enable in all rasterization steps to avoid jagged edges at small sizes
- **Transparency:** SVG has no transparency (solid gradient background); `.ico` can have alpha channel for future extensibility

## 8. Size & Deployment

- **Windows installer (NSIS/MSI):** Shows 32×32 icon in app list
- **Desktop shortcut:** Windows typically uses 32×32 on taskbar, 16×16 on labels
- **Tauri bundle config:** `src-tauri/tauri.conf.json` points to `src-tauri/icons/icon.ico`; no changes needed post-replacement

## 9. Success Criteria

- ✓ Icon is instantly recognizable as "network topology"
- ✓ Legible at 32×32 (smallest reasonable deployment size)
- ✓ Color palette is distinctive from other Microsoft / Azure tooling (orange + saturated 6-color palette)
- ✓ Matches app's own UI aesthetic (blue/navy/green color echoes from the graph canvas)
- ✓ `.ico` file integrates with existing Tauri build without config changes

## 10. Next Steps

1. Create SVG master (`az-plotter-icon.svg`)
2. Rasterize to 5 PNG sizes
3. Composite into Windows `.ico`
4. Replace `src-tauri/icons/icon.ico`
5. Test in installer (build step should pick it up automatically)
6. Verify at 32×32 and 16×16 in taskbar / app list
