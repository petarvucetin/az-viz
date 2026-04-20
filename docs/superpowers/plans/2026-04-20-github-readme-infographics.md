# GitHub README Infographics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a README.md at repo root with 6 hand-authored inline SVG infographics that communicate what az-plotter does in under 5 seconds for visitors and explain its architecture for prospective contributors — no JS, no animation, no external build step.

**Architecture:** 6 standalone SVG files in `docs/readme/` (each self-contained with embedded `<style>` and baked color palette), plus `README.md` at repo root referencing them via `<img>` tags. Zero runtime code. A small Node validation script in `scripts/` guards against broken asset paths and stale version badges.

**Tech Stack:** Hand-authored SVG (XML), markdown, Node (for one validation script), system font stacks only (no external font loads).

**Related spec:** `docs/superpowers/specs/2026-04-20-github-readme-infographics-design.md`

---

## File Inventory

**Created:**
- `docs/readme/hero.svg` — 1200×420, terminal-to-graph transformation
- `docs/readme/pipeline.svg` — 1100×180, 5-stage architecture pipeline
- `docs/readme/tile-parse.svg` — 260×180, feature tile
- `docs/readme/tile-validate.svg` — 260×180, feature tile
- `docs/readme/tile-visualize.svg` — 260×180, feature tile
- `docs/readme/tile-run.svg` — 260×180, feature tile
- `README.md` — repo root
- `scripts/validate-readme-assets.js` — asset path + version consistency check

**Reused (no modification):**
- `docs/screenshot.png` — existing screenshot, embedded under "See it running"

**Modified:** none

---

## Design Tokens (used in every SVG)

| Token | Hex | Use |
|---|---|---|
| `bg` | `#0d1117` | canvas background |
| `surface` | `#161b22` | card/pill fills |
| `stroke` | `#30363d` | subtle borders |
| `text` | `#C9D1D9` | primary text |
| `text-dim` | `#8B949E` | captions |
| `green` | `#3FB950` | terminal/parse |
| `green-dim` | `#238636` | prompt |
| `azure` | `#0078D4` | graph/run |
| `azure-dim` | `#005A9E` | edges |

Fonts:
- Code: `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`
- Prose: `system-ui, -apple-system, "Segoe UI", sans-serif`

---

## Task 1: Create `docs/readme/` directory

**Files:**
- Create: `docs/readme/.gitkeep` (placeholder so git tracks the empty dir until assets land)

- [ ] **Step 1: Create the directory with a .gitkeep**

```bash
mkdir -p docs/readme
touch docs/readme/.gitkeep
```

- [ ] **Step 2: Verify**

```bash
ls -la docs/readme/
```
Expected: directory exists, contains `.gitkeep`.

- [ ] **Step 3: Commit**

```bash
git add docs/readme/.gitkeep
git commit -m "docs: create docs/readme/ for infographic assets"
```

---

## Task 2: Author `hero.svg`

**Files:**
- Create: `docs/readme/hero.svg`

- [ ] **Step 1: Write the full SVG**

Write this exact content to `docs/readme/hero.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 460" width="1200" height="460" role="img" aria-label="Author Azure infrastructure in az CLI. See it as a graph.">
  <style>
    .bg { fill: #0d1117; }
    .surface { fill: #161b22; }
    .stroke { stroke: #30363d; }
    .text { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
    .dim { fill: #8B949E; }
    .code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 13px; }
    .prompt { fill: #238636; }
    .kw { fill: #3FB950; }
    .arg { fill: #C9D1D9; }
    .opt { fill: #8B949E; }
    .title { font-size: 22px; font-weight: 600; }
    .wordmark { font-size: 12px; fill: #8B949E; letter-spacing: 0.12em; text-transform: uppercase; }
    .node-fill { fill: #161b22; }
    .node-stroke { stroke: #0078D4; stroke-width: 1.5; fill: none; }
    .group-frame { fill: rgba(22, 27, 34, 0.6); stroke: #30363d; stroke-width: 1; stroke-dasharray: 4 3; }
    .edge { fill: none; stroke: #005A9E; stroke-width: 1.5; }
    .node-label { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 10px; fill: #C9D1D9; }
    .group-label { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 10px; fill: #8B949E; }
  </style>

  <defs>
    <linearGradient id="arrow-grad" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="#3FB950"/>
      <stop offset="1" stop-color="#0078D4"/>
    </linearGradient>
    <marker id="arrow-head" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
      <path d="M0,0 L10,5 L0,10 Z" fill="#0078D4"/>
    </marker>
  </defs>

  <!-- Canvas -->
  <rect x="0" y="0" width="1200" height="420" rx="12" class="bg"/>

  <!-- === LEFT: terminal window === -->
  <g transform="translate(24, 24)">
    <rect x="0" y="0" width="520" height="372" rx="8" class="surface stroke" stroke-width="1"/>
    <!-- title bar -->
    <rect x="0" y="0" width="520" height="28" rx="8" class="surface"/>
    <rect x="0" y="20" width="520" height="8" class="surface"/>
    <circle cx="16" cy="14" r="5" fill="#ff5f56"/>
    <circle cx="34" cy="14" r="5" fill="#ffbd2e"/>
    <circle cx="52" cy="14" r="5" fill="#27c93f"/>
    <text x="260" y="18" text-anchor="middle" class="dim code">az-commands.txt</text>

    <!-- command lines -->
    <g transform="translate(16, 52)">
      <g transform="translate(0, 0)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az</tspan> <tspan class="arg">group create</tspan> <tspan class="opt">-n</tspan> <tspan class="arg">rg-prod</tspan> <tspan class="opt">-l</tspan> <tspan class="arg">westeurope</tspan></text>
      </g>
      <g transform="translate(0, 44)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az network vnet</tspan> <tspan class="arg">create</tspan> <tspan class="opt">-g</tspan> <tspan class="arg">rg-prod</tspan> <tspan class="opt">-n</tspan> <tspan class="arg">vnet-core</tspan></text>
        <text x="0" y="16" class="code opt">    --address-prefixes 10.0.0.0/16</text>
      </g>
      <g transform="translate(0, 104)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az network vnet subnet</tspan> <tspan class="arg">create</tspan> <tspan class="opt">-g</tspan> <tspan class="arg">rg-prod</tspan></text>
        <text x="0" y="16" class="code opt">    --vnet-name vnet-core -n snet-app</text>
      </g>
      <g transform="translate(0, 164)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az network private-dns zone</tspan> <tspan class="arg">create</tspan></text>
        <text x="0" y="16" class="code opt">    -g rg-prod -n privatelink.blob.core.windows.net</text>
      </g>
      <g transform="translate(0, 224)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az network private-dns link vnet</tspan> <tspan class="arg">create</tspan></text>
        <text x="0" y="16" class="code opt">    -n blob-link --zone-name privatelink.blob... </text>
      </g>
      <g transform="translate(0, 284)">
        <text class="code"><tspan class="prompt">$</tspan> <tspan class="kw">az network private-endpoint</tspan> <tspan class="arg">create</tspan></text>
        <text x="0" y="16" class="code opt">    -n pe-storage --subnet snet-app ...</text>
      </g>
    </g>
  </g>

  <!-- === MIDDLE: arrow === -->
  <g transform="translate(560, 180)">
    <line x1="0" y1="30" x2="72" y2="30" stroke="url(#arrow-grad)" stroke-width="3" stroke-dasharray="6 4" marker-end="url(#arrow-head)"/>
    <text x="40" y="62" text-anchor="middle" class="wordmark">az-plotter</text>
  </g>

  <!-- === RIGHT: mini-graph === -->
  <g transform="translate(656, 40)">
    <!-- group frame 1: rg-prod / vnet-core -->
    <rect x="0" y="0" width="260" height="160" rx="10" class="group-frame"/>
    <text x="14" y="20" class="group-label">rg-prod / vnet-core</text>
    <!-- node: vnet -->
    <rect x="18" y="40" width="100" height="36" rx="6" class="node-fill node-stroke"/>
    <text x="68" y="62" text-anchor="middle" class="node-label">vnet</text>
    <!-- node: subnet -->
    <rect x="142" y="40" width="100" height="36" rx="6" class="node-fill node-stroke"/>
    <text x="192" y="62" text-anchor="middle" class="node-label">subnet</text>
    <!-- edge -->
    <path d="M118 58 C 130 58, 130 58, 142 58" class="edge"/>
    <!-- node: pe -->
    <rect x="80" y="100" width="100" height="36" rx="6" class="node-fill node-stroke"/>
    <text x="130" y="122" text-anchor="middle" class="node-label">private-endpoint</text>
    <path d="M192 76 C 192 88, 130 88, 130 100" class="edge"/>

    <!-- group frame 2: rg-prod / dns -->
    <rect x="280" y="0" width="240" height="160" rx="10" class="group-frame"/>
    <text x="294" y="20" class="group-label">rg-prod / private-dns</text>
    <!-- node: zone -->
    <rect x="298" y="40" width="120" height="36" rx="6" class="node-fill node-stroke"/>
    <text x="358" y="62" text-anchor="middle" class="node-label">privatelink.blob</text>
    <!-- node: link -->
    <rect x="298" y="100" width="120" height="36" rx="6" class="node-fill node-stroke"/>
    <text x="358" y="122" text-anchor="middle" class="node-label">blob-link</text>
    <path d="M358 76 L 358 100" class="edge"/>
    <!-- cross-group edge: link -> vnet -->
    <path d="M298 118 C 260 118, 260 58, 242 58" class="edge"/>
  </g>

  <!-- === Tagline === -->
  <text x="600" y="444" text-anchor="middle" class="text title">Author Azure infrastructure in <tspan class="code" style="font-size: 20px;">az</tspan> CLI. See it as a graph.</text>
</svg>
```

- [ ] **Step 2: Open it in a browser to verify**

```bash
start docs/readme/hero.svg  # Windows
# or: open docs/readme/hero.svg (mac) / xdg-open (linux)
```

Expected: renders a dark-canvas image with the terminal on the left, an arrow in the middle, a mini-graph on the right, and the tagline below. No JS errors. All text legible.

- [ ] **Step 3: Commit**

```bash
git add docs/readme/hero.svg
git commit -m "docs: add README hero infographic"
```

---

## Task 3: Author `pipeline.svg`

**Files:**
- Create: `docs/readme/pipeline.svg`

- [ ] **Step 1: Write the full SVG**

Write this exact content to `docs/readme/pipeline.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1100 180" width="1100" height="180" role="img" aria-label="Pipeline: tokenize, parse, plan, layout, run or emit.">
  <style>
    .bg { fill: #0d1117; }
    .text { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
    .dim { fill: #8B949E; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
    .stage-text { font-size: 14px; font-weight: 600; }
    .sub-text { font-size: 11px; }
    .heading { fill: #8B949E; font-size: 11px; letter-spacing: 0.14em; text-transform: uppercase; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; }
    .pill-stroke { stroke: #30363d; stroke-width: 1; }
    .connector { stroke: #30363d; stroke-width: 2; fill: none; }
  </style>
  <defs>
    <linearGradient id="pipe-grad" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0"    stop-color="#3FB950"/>
      <stop offset="0.25" stop-color="#238636"/>
      <stop offset="0.5"  stop-color="#161b22"/>
      <stop offset="0.75" stop-color="#005A9E"/>
      <stop offset="1"    stop-color="#0078D4"/>
    </linearGradient>
    <marker id="conn-head" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
      <path d="M0,0 L10,5 L0,10 Z" fill="#30363d"/>
    </marker>
  </defs>

  <rect width="1100" height="180" rx="12" class="bg"/>

  <text x="40" y="36" class="heading">pipeline</text>

  <!-- Gradient strip behind the pills -->
  <rect x="60" y="72" width="980" height="60" rx="30" fill="url(#pipe-grad)" opacity="0.18"/>

  <!-- Stage pills -->
  <!-- 5 pills at x = 60, 252, 444, 636, 828; each 180 wide, 60 tall -->
  <g>
    <rect x="60"  y="72" width="180" height="60" rx="30" fill="#161b22" class="pill-stroke"/>
    <text x="150" y="100" text-anchor="middle" class="text stage-text">tokenize</text>
    <text x="150" y="118" text-anchor="middle" class="dim sub-text">lex az tokens</text>
  </g>
  <path d="M240 102 L 252 102" class="connector" marker-end="url(#conn-head)"/>

  <g>
    <rect x="252" y="72" width="180" height="60" rx="30" fill="#161b22" class="pill-stroke"/>
    <text x="342" y="100" text-anchor="middle" class="text stage-text">parse</text>
    <text x="342" y="118" text-anchor="middle" class="dim sub-text">AST + refs</text>
  </g>
  <path d="M432 102 L 444 102" class="connector" marker-end="url(#conn-head)"/>

  <g>
    <rect x="444" y="72" width="180" height="60" rx="30" fill="#161b22" class="pill-stroke"/>
    <text x="534" y="100" text-anchor="middle" class="text stage-text">plan</text>
    <text x="534" y="118" text-anchor="middle" class="dim sub-text">topo order</text>
  </g>
  <path d="M624 102 L 636 102" class="connector" marker-end="url(#conn-head)"/>

  <g>
    <rect x="636" y="72" width="180" height="60" rx="30" fill="#161b22" class="pill-stroke"/>
    <text x="726" y="100" text-anchor="middle" class="text stage-text">layout</text>
    <text x="726" y="118" text-anchor="middle" class="dim sub-text">ELK graph</text>
  </g>
  <path d="M816 102 L 828 102" class="connector" marker-end="url(#conn-head)"/>

  <g>
    <rect x="828" y="72" width="212" height="60" rx="30" fill="#161b22" class="pill-stroke"/>
    <text x="934" y="100" text-anchor="middle" class="text stage-text">run / emit</text>
    <text x="934" y="118" text-anchor="middle" class="dim sub-text">live az | script</text>
  </g>
</svg>
```

- [ ] **Step 2: Open in a browser to verify**

```bash
start docs/readme/pipeline.svg
```

Expected: a horizontal strip of 5 pill-shaped stages with a subtle green→azure gradient band behind them, connectors between each, all labels legible.

- [ ] **Step 3: Commit**

```bash
git add docs/readme/pipeline.svg
git commit -m "docs: add README architecture pipeline diagram"
```

---

## Task 4: Author the 4 feature tiles

Each tile is a 260×180 card with an icon, label, and one-line caption. Four small files, same template, different icon + copy.

**Files:**
- Create: `docs/readme/tile-parse.svg`
- Create: `docs/readme/tile-validate.svg`
- Create: `docs/readme/tile-visualize.svg`
- Create: `docs/readme/tile-run.svg`

- [ ] **Step 1: Write `tile-parse.svg`**

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 260 180" width="260" height="180" role="img" aria-label="Parse: tokenizes az commands into a typed AST.">
  <style>
    .bg { fill: #0d1117; }
    .card { fill: #161b22; stroke: #30363d; stroke-width: 1; }
    .label { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 16px; font-weight: 600; }
    .sub { fill: #8B949E; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 11px; }
    .icon { fill: none; stroke: #0078D4; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
  </style>
  <rect width="260" height="180" class="bg"/>
  <rect x="8" y="8" width="244" height="164" rx="12" class="card"/>
  <!-- Icon: terminal cursor > in a rounded square -->
  <g transform="translate(110, 30)">
    <rect x="0" y="0" width="40" height="32" rx="6" class="icon"/>
    <path d="M10 10 L 18 16 L 10 22" class="icon"/>
    <line x1="22" y1="22" x2="30" y2="22" class="icon"/>
  </g>
  <text x="130" y="104" text-anchor="middle" class="label">Parse</text>
  <text x="130" y="128" text-anchor="middle" class="sub">Tokenizes <tspan style="font-family: ui-monospace, monospace;">az</tspan> commands into</text>
  <text x="130" y="144" text-anchor="middle" class="sub">a typed AST — 15 subcommands.</text>
</svg>
```

- [ ] **Step 2: Write `tile-validate.svg`**

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 260 180" width="260" height="180" role="img" aria-label="Validate: catches broken references before you run.">
  <style>
    .bg { fill: #0d1117; }
    .card { fill: #161b22; stroke: #30363d; stroke-width: 1; }
    .label { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 16px; font-weight: 600; }
    .sub { fill: #8B949E; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 11px; }
    .icon { fill: none; stroke: #0078D4; stroke-width: 2.5; stroke-linecap: round; stroke-linejoin: round; }
  </style>
  <rect width="260" height="180" class="bg"/>
  <rect x="8" y="8" width="244" height="164" rx="12" class="card"/>
  <!-- Icon: check in a circle -->
  <g transform="translate(110, 26)">
    <circle cx="20" cy="20" r="18" class="icon"/>
    <path d="M12 20 L 18 26 L 28 14" class="icon"/>
  </g>
  <text x="130" y="104" text-anchor="middle" class="label">Validate</text>
  <text x="130" y="128" text-anchor="middle" class="sub">Catches broken references</text>
  <text x="130" y="144" text-anchor="middle" class="sub">before you run.</text>
</svg>
```

- [ ] **Step 3: Write `tile-visualize.svg`**

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 260 180" width="260" height="180" role="img" aria-label="Visualize: auto-layout via ELK, draggable, persistent.">
  <style>
    .bg { fill: #0d1117; }
    .card { fill: #161b22; stroke: #30363d; stroke-width: 1; }
    .label { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 16px; font-weight: 600; }
    .sub { fill: #8B949E; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 11px; }
    .node { fill: #0d1117; stroke: #0078D4; stroke-width: 2; }
    .edge { fill: none; stroke: #0078D4; stroke-width: 1.5; }
  </style>
  <rect width="260" height="180" class="bg"/>
  <rect x="8" y="8" width="244" height="164" rx="12" class="card"/>
  <!-- Icon: 3 connected nodes -->
  <g transform="translate(102, 26)">
    <circle cx="10" cy="10" r="7" class="node"/>
    <circle cx="46" cy="10" r="7" class="node"/>
    <circle cx="28" cy="38" r="7" class="node"/>
    <line x1="17" y1="10" x2="39" y2="10" class="edge"/>
    <line x1="13" y1="17" x2="24" y2="32" class="edge"/>
    <line x1="43" y1="17" x2="32" y2="32" class="edge"/>
  </g>
  <text x="130" y="104" text-anchor="middle" class="label">Visualize</text>
  <text x="130" y="128" text-anchor="middle" class="sub">Auto-layout via ELK.</text>
  <text x="130" y="144" text-anchor="middle" class="sub">Draggable. Persistent.</text>
</svg>
```

- [ ] **Step 4: Write `tile-run.svg`**

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 260 180" width="260" height="180" role="img" aria-label="Run: execute live against Azure or emit as a script.">
  <style>
    .bg { fill: #0d1117; }
    .card { fill: #161b22; stroke: #30363d; stroke-width: 1; }
    .label { fill: #C9D1D9; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 16px; font-weight: 600; }
    .sub { fill: #8B949E; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; font-size: 11px; }
    .icon { fill: #0078D4; }
  </style>
  <rect width="260" height="180" class="bg"/>
  <rect x="8" y="8" width="244" height="164" rx="12" class="card"/>
  <!-- Icon: play triangle -->
  <g transform="translate(110, 22)">
    <path d="M4 4 L 36 22 L 4 40 Z" class="icon"/>
  </g>
  <text x="130" y="104" text-anchor="middle" class="label">Run</text>
  <text x="130" y="128" text-anchor="middle" class="sub">Execute live against Azure,</text>
  <text x="130" y="144" text-anchor="middle" class="sub">or emit as a script.</text>
</svg>
```

- [ ] **Step 5: Open all four in a browser and verify**

```bash
start docs/readme/tile-parse.svg
start docs/readme/tile-validate.svg
start docs/readme/tile-visualize.svg
start docs/readme/tile-run.svg
```

Expected: 4 uniform dark-canvas cards, each with a blue icon centered near the top, bold label, dim two-line caption. No visual breaks. Consistent card stroke and corner radius.

- [ ] **Step 6: Commit**

```bash
git add docs/readme/tile-parse.svg docs/readme/tile-validate.svg docs/readme/tile-visualize.svg docs/readme/tile-run.svg
git commit -m "docs: add 4 README feature tiles"
```

---

## Task 5: Write `README.md`

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write README.md**

Write this exact content to `README.md` at repo root:

````markdown
# az-plotter

<p align="center">
  <img src="docs/readme/hero.svg" alt="Write az CLI commands, see them as an Azure resource graph" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.1.65-0078D4?style=flat-square" alt="version 0.1.65" />
  <img src="https://img.shields.io/badge/license-MIT-3FB950?style=flat-square" alt="MIT license" />
  <img src="https://img.shields.io/badge/platform-Windows-161b22?style=flat-square" alt="Windows" />
  <img src="https://img.shields.io/badge/built%20with-Tauri-0078D4?style=flat-square" alt="Built with Tauri" />
</p>

A Windows desktop app that parses `az` CLI commands into a typed model of your Azure infrastructure, lays it out as an interactive graph, and either runs it live against Azure or emits an idempotent script. Built with Tauri, Svelte, and Rust.

## What it does

<table>
  <tr>
    <td align="center"><img src="docs/readme/tile-parse.svg"     width="240" alt="Parse" /></td>
    <td align="center"><img src="docs/readme/tile-validate.svg"  width="240" alt="Validate" /></td>
    <td align="center"><img src="docs/readme/tile-visualize.svg" width="240" alt="Visualize" /></td>
    <td align="center"><img src="docs/readme/tile-run.svg"       width="240" alt="Run" /></td>
  </tr>
</table>

## How it works

<p align="center">
  <img src="docs/readme/pipeline.svg" alt="tokenize → parse → plan → layout → run/emit" />
</p>

The static pipeline runs left-to-right. The **tokenizer** splits raw `az` invocations into structured tokens. The **parser** builds a typed AST and resolves cross-command references (e.g. a private-endpoint pointing to a subnet declared four lines earlier). The **planner** orders commands topologically so dependencies always come before dependents. The **layout** stage hands that graph to ELK, which produces coordinates for the Svelte Flow renderer. By the time it hits the screen, every edge represents a real Azure dependency.

At runtime, az-plotter can either execute the plan live — dispatching each command through the local `az` CLI and streaming stdout/stderr into the UI — or emit a plain shell script for use elsewhere. A verification subsystem (`src-tauri/src/verify/`) that will cross-check the graph against real Azure state is scaffolded but not yet wired up.

## See it running

<p align="center">
  <img src="docs/screenshot.png" width="900" alt="az-plotter UI showing an Azure resource graph" />
</p>

## Install

1. Grab the latest `.msi` (WiX) or `.exe` (NSIS) installer from [Releases](../../releases).
2. Run it. The NSIS installer is per-user and requires no admin rights.
3. Launch **az-plotter** from the Start menu.

## Build from source

Prerequisites: Rust toolchain, Node.js 18+, and the [Tauri v1 prereqs](https://tauri.app/v1/guides/getting-started/prerequisites) (MSVC build tools + WebView2 on Windows).

```bash
git clone https://github.com/<owner>/az-viz-web.git
cd az-viz-web
npm --prefix ui install
cargo tauri dev          # run in development
cargo tauri build        # produce MSI and NSIS installers
```

Release artifacts land in `target/release/bundle/{msi,nsis}/`.

## License

MIT.
````

- [ ] **Step 2: Preview locally**

If `grip` is installed: `grip README.md` and open the URL it prints.
Otherwise open `README.md` directly on GitHub after pushing (Task 7).

Expected: hero SVG at top, badges row, pitch paragraph, 4-tile table, pipeline image, two explanatory paragraphs, screenshot, install/build sections, license.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add repo README with SVG infographics"
```

---

## Task 6: Add asset-validation script

**Files:**
- Create: `scripts/validate-readme-assets.js`

This script does two things, both cheap:
1. Parses `README.md`, extracts every `src="..."` image path, and fails if any referenced file doesn't exist.
2. Reads the version from `src-tauri/tauri.conf.json` and fails if the version-badge shield URL in README doesn't match.

- [ ] **Step 1: Write the validation script**

```javascript
#!/usr/bin/env node
// Validates that all image assets referenced by README.md exist on disk,
// and that the version badge matches tauri.conf.json.

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const readmePath = path.join(repoRoot, 'README.md');
const tauriConfPath = path.join(repoRoot, 'src-tauri', 'tauri.conf.json');

const readme = fs.readFileSync(readmePath, 'utf8');

let errors = 0;

// 1. Verify every local src="..." path exists.
const srcRe = /src="([^"]+)"/g;
for (const m of readme.matchAll(srcRe)) {
  const src = m[1];
  if (src.startsWith('http://') || src.startsWith('https://')) continue;
  const abs = path.join(repoRoot, src);
  if (!fs.existsSync(abs)) {
    console.error(`[readme] missing asset: ${src}`);
    errors++;
  }
}

// 2. Verify the version badge matches tauri.conf.json.
const conf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
const version = conf.package && conf.package.version;
if (!version) {
  console.error('[readme] could not read version from tauri.conf.json');
  errors++;
} else {
  const badgeRe = /shields\.io\/badge\/version-([^-]+)-/;
  const m = readme.match(badgeRe);
  if (!m) {
    console.error('[readme] version badge not found');
    errors++;
  } else if (m[1] !== version) {
    console.error(`[readme] version badge mismatch: badge=${m[1]} tauri.conf=${version}`);
    errors++;
  }
}

if (errors > 0) {
  console.error(`[readme] ${errors} error(s)`);
  process.exit(1);
}
console.log('[readme] OK');
```

- [ ] **Step 2: Run it and verify it passes**

```bash
node scripts/validate-readme-assets.js
```

Expected stdout: `[readme] OK`
Expected exit code: 0

- [ ] **Step 3: Deliberately break it to confirm it fails loudly**

Temporarily rename `docs/readme/hero.svg` to `hero.svg.bak` and re-run:

```bash
mv docs/readme/hero.svg docs/readme/hero.svg.bak
node scripts/validate-readme-assets.js
# Expected: "[readme] missing asset: docs/readme/hero.svg" and exit 1
mv docs/readme/hero.svg.bak docs/readme/hero.svg
```

Re-run and confirm it's back to OK.

- [ ] **Step 4: Commit**

```bash
git add scripts/validate-readme-assets.js
git commit -m "docs: add README asset-and-version validation script"
```

---

## Task 7: End-to-end validation

- [ ] **Step 1: Run the validator one more time**

```bash
node scripts/validate-readme-assets.js
```
Expected: `[readme] OK`

- [ ] **Step 2: Open the 6 SVGs and confirm visuals**

Open each in a browser and walk through the visual checklist below. For each SVG, verify:

- Background is dark (`#0d1117`), not white.
- All text is legible at 100% zoom.
- No missing glyphs (placeholder boxes indicate a font failed — fall back to system fonts only).
- Colors match the token palette (green `#3FB950`, azure `#0078D4`, dim gray `#8B949E`).

If any SVG shows rendering issues in your browser (especially Edge/Chromium on Windows, which is what most GitHub visitors use), fix inline before pushing.

- [ ] **Step 3: Push and verify on GitHub**

```bash
git push
```

Open the repo on GitHub in **dark theme** and confirm:

- Hero SVG renders at full width of the README column, terminal + arrow + graph all visible.
- Badge row renders (shields.io badges are SVG served from their CDN; they do not require commit).
- The 4-tile table lays out as 4 columns on desktop (may wrap to 2 on narrow viewports — acceptable).
- Pipeline diagram renders end-to-end.
- Screenshot displays.

Then switch to **light theme** and confirm:

- Dark-canvas SVGs are visibly a dark rectangle on a light page but text is still legible. This is expected — we tuned for dark, not dual-theme.

- [ ] **Step 4: Mobile check**

Open the GitHub repo page on a phone (or narrow your browser to ~400px). Confirm:

- Hero SVG scales down proportionally, text still readable at ~0.35x.
- If hero text becomes unreadable, note it as future-work but don't block — README mobile readers are a minority.

- [ ] **Step 5: Final cleanup commit (if any tweaks needed)**

If any SVG needed visual adjustments in Step 2, commit those:

```bash
git add docs/readme/
git commit -m "docs: tune README SVG visuals after render validation"
git push
```

---

## Self-Review Checklist (for the implementing engineer)

Before declaring done:

1. **Spec coverage:** every deliverable listed in the spec's "Asset inventory" section has been created. Verify by `ls docs/readme/` — should show 6 SVGs + `.gitkeep`.
2. **No placeholders:** no `TBD`, `TODO`, or `<!-- fill in -->` strings in any committed file: `grep -RIn 'TBD\|TODO\|FIXME' README.md docs/readme/ scripts/validate-readme-assets.js` should return empty (there is one `TBD` in the spec — that's fine, specs are allowed to flag deferred items).
3. **Asset paths match:** `node scripts/validate-readme-assets.js` passes.
4. **Version consistency:** badge in README matches `src-tauri/tauri.conf.json` (the validator checks this, but eyeball it too).
5. **Visual consistency:** the 4 tiles have the same card shape, same corner radius, same icon area size, same text placement. Open them side-by-side.
6. **No animation:** search all SVGs for `<animate`, `@keyframes`, or `animation:`; should be zero hits. (User preference: zero animation.)

```bash
grep -RIn 'animate\|keyframes\|animation:' docs/readme/ | grep -v aria-label
# Expected: no output
```
