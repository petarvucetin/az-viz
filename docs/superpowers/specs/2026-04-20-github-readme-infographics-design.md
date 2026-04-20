# GitHub README Infographics — Design Spec

**Date:** 2026-04-20
**Status:** Approved (pending final spec review)
**Owner:** petar

## Goal

Give first-time GitHub visitors an immediate, visual understanding of what az-plotter is, and give prospective contributors a compact architectural overview — both in a single `README.md` at the repo root. No separate website, no build step, no JavaScript.

## Non-Goals

- No GitHub Pages site.
- No interactive or JS-driven content. GitHub strips `<script>` from both README-embedded and image-referenced SVG.
- No animation (explicit user preference).
- No bundler or codegen for SVG assets. Files are hand-authored and committed as-is.
- No dual light/dark theme switching. One palette tuned for GitHub-dark canvas, acceptable contrast on light.
- No Azure official icon pack (would require attribution and risks implying Microsoft affiliation).

## Audience

Both addressed by a single scrolling README:

1. **First-time visitors** — the top half of the README (hero + pitch + feature tiles + screenshot) answers "what is this, should I try it?"
2. **Prospective contributors** — the lower half (pipeline diagram + build-from-source instructions) answers "how does it work internally?"

## Thesis

The hero infographic must communicate one idea in under 5 seconds: **write `az` CLI commands, see them as an Azure resource graph.** All other content is secondary.

## Deliverables

### New files

```
README.md                          — repo root, new file
docs/readme/hero.svg               — ~1200×420, terminal→graph transformation
docs/readme/pipeline.svg           — ~1100×180, 5-stage architecture
docs/readme/tile-parse.svg         — 260×180, feature tile
docs/readme/tile-validate.svg      — 260×180, feature tile
docs/readme/tile-visualize.svg     — 260×180, feature tile
docs/readme/tile-run.svg           — 260×180, feature tile
```

### Reused files

- `docs/screenshot.png` — embedded in the "See it running" section as a real-product reference shot.

## Constraints (GitHub rendering)

- **No `<script>`.** Stripped from both README-inline and `<img>`-referenced SVG.
- **No SMIL, no CSS `@keyframes`.** User preference: zero animation.
- **No external fonts.** GitHub's image proxy does not honor cross-origin font fetches. Use system font stacks only: `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace` for code, `system-ui, -apple-system, Segoe UI, sans-serif` for prose.
- **No `<foreignObject>` with HTML.** Stripped.
- **Inline `<style>` inside SVG is honored** when the SVG is referenced via `<img>` or `![]()`. Use this for color tokens and text styling.
- **Width control in README:** prefer `<img src="..." width="N">` over markdown `![]()` for infographics that need size constraints.

## Design tokens (baked into each SVG, no shared stylesheet)

| Token | Hex | Use |
|---|---|---|
| `bg` | `#0d1117` | SVG canvas background |
| `surface` | `#161b22` | card/pill fills |
| `stroke` | `#30363d` | subtle borders |
| `text` | `#C9D1D9` | primary text |
| `text-dim` | `#8B949E` | captions |
| `green` | `#3FB950` | terminal / parse stage |
| `green-dim` | `#238636` | prompt character |
| `azure` | `#0078D4` | graph nodes / run stage |
| `azure-dim` | `#005A9E` | edge strokes |

All SVG canvases are `#0d1117` with 12px rounded corners (via `<rect>` with `rx="12"`), not transparent — because the same asset must also look deliberate if someone opens the file directly.

## Asset specs

### `hero.svg` (1200×420)

**Layout:**

- `[0–560]` — terminal window mock. macOS-style traffic-light dots (red/yellow/green circles) in the top-left corner. Monospaced content, 6 lines of real `az` commands sourced from the project's existing test/example corpus:

  ```
  $ az group create -n rg-prod -l westeurope
  $ az network vnet create -g rg-prod -n vnet-core --address-prefixes 10.0.0.0/16
  $ az network vnet subnet create -g rg-prod --vnet-name vnet-core -n snet-app --address-prefix 10.0.1.0/24
  $ az network private-dns zone create -g rg-prod -n privatelink.blob.core.windows.net
  $ az network private-dns link vnet create -g rg-prod -n blob-link --zone-name privatelink.blob.core.windows.net --virtual-network vnet-core --registration-enabled false
  $ az network private-endpoint create -g rg-prod -n pe-storage --vnet-name vnet-core --subnet snet-app --private-connection-resource-id <id> --group-id blob --connection-name c1
  ```

  Colors: `green-dim` for `$` prompt, `green` for the `az` keyword, `text` for everything else.

- `[560–640]` — arrow zone. Static dashed arrow, stroke gradient `green → azure` baked in via `<linearGradient>`. Centered below the arrow: a small "az-plotter" wordmark at 12pt `text-dim`.

- `[640–1200]` — mini-graph. Two rounded `<rect>` frames (semi-transparent `surface` fill, `stroke` border) representing resource groups, containing 4 rounded resource nodes with `azure` accent strokes. Bezier edges (`<path d="M... C...">`) between nodes using `azure-dim`. Visual language matches the real Svelte Flow rendering in the app.

**Caption below the SVG canvas (inside the same SVG):** 22pt `text` — *"Author Azure infrastructure in `az` CLI. See it as a graph."*

### `pipeline.svg` (1100×180)

Five pill-shaped stages connected by arrow segments:

```
tokenize → parse → plan → layout → run/emit
```

Each pill is a 180×60 rounded `<rect>` with:
- Label line (stage name) in 14pt `text`
- Subtext (one line) in 11pt `text-dim`

Subtexts:
- tokenize: `lex az tokens`
- parse: `AST + refs`
- plan: `topo order`
- layout: `ELK graph`
- run/emit: `live az | script`

Pill fill color shifts from `green` (left) through `surface` (middle) to `azure` (right) via a baked linear gradient. Arrow connectors are plain static paths, 2px `text-dim`.

### Feature tiles (4 × 260×180)

Each tile:
- 12px rounded `<rect>`, `surface` fill, `stroke` border.
- Top-center icon, ~40×40, single-color `azure`, hand-drawn geometric:
  - `tile-parse.svg`: terminal-cursor `>` inside a box
  - `tile-validate.svg`: checkmark inside a circle
  - `tile-visualize.svg`: three connected nodes (graph glyph)
  - `tile-run.svg`: filled play triangle
- Label in 14pt `text`, centered below the icon.
- Subtext in 11pt `text-dim`, two-line max, centered below the label.

Subtexts:
- Parse: *"Tokenizes `az` commands into a typed AST — 15 subcommands and growing."*
- Validate: *"Catches broken references before you run."*
- Visualize: *"Auto-layout via ELK. Draggable. Persistent."*
- Run: *"Execute live against Azure, or emit as a script."*

(Subcommand count verified against `src-tauri/arg-map.json` at spec time: 15. Update the number in the tile when new subcommands are added, or keep it coarse ("10+") if sync friction becomes an issue.)

## README.md structure

```markdown
# az-plotter

<img src="docs/readme/hero.svg" alt="Write az CLI commands, see an Azure resource graph" />

[badges row — 4 shields.io badges: version, license, platform, built-with-Tauri]

> One paragraph, ~50 words, direct/technical tone.
> Example: "A Windows desktop app that parses `az` CLI commands into a typed
> model of your Azure infrastructure, lays it out as an interactive graph,
> and either runs it live against Azure or emits an idempotent script. Built
> with Tauri, Svelte, and Rust."

## What it does

<table>
  <tr>
    <td><img src="docs/readme/tile-parse.svg"      width="260" /></td>
    <td><img src="docs/readme/tile-validate.svg"   width="260" /></td>
    <td><img src="docs/readme/tile-visualize.svg"  width="260" /></td>
    <td><img src="docs/readme/tile-run.svg"        width="260" /></td>
  </tr>
</table>

## How it works

<img src="docs/readme/pipeline.svg" alt="tokenize → parse → plan → layout → run/emit" />

Two short paragraphs (~60 words each):
- First: the static pipeline (tokenize → parse → plan → layout).
  Ends with "...by the time it hits the screen, every edge represents a
  real Azure dependency."
- Second: the runtime split (live Azure vs emit-as-script), plus a
  pointer to the `verify/` subsystem as future work.

## See it running

<img src="docs/screenshot.png" width="900" alt="az-plotter UI screenshot" />

## Install

Three lines: grab the latest .msi or .exe from Releases, run it, done.

## Build from source

Five lines: clone, `npm --prefix ui install`, `cargo tauri dev` (or
`cargo tauri build` for installers). Link to the existing
`docs/` directory for architecture notes.

## License

MIT.
```

### Badge row (specific)

Use shields.io static badges (rendered server-side as SVG, no JS):

- `![version](https://img.shields.io/badge/version-0.1.65-0078D4?style=flat-square)` — manually bumped alongside version changes
- `![license](https://img.shields.io/badge/license-MIT-3FB950?style=flat-square)`
- `![platform](https://img.shields.io/badge/platform-Windows-161b22?style=flat-square)`
- `![Tauri](https://img.shields.io/badge/built%20with-Tauri-0078D4?style=flat-square)`

Version badge staleness is acceptable — it's informational, not load-bearing.

## Validation plan

Before pushing:

1. Open each SVG file directly in a browser (`file:///...`) — verify it renders standalone and colors match the token table.
2. Render the README locally via `grip` (or equivalent) and confirm all 7 SVGs load, sizing is consistent, and no images break.
3. After initial commit and push, visit the GitHub rendering of README.md in both dark and light theme — confirm the dark-tuned SVGs still read acceptably on light canvas.
4. Verify on mobile GitHub (responsive): SVGs scale down cleanly. The hero's 1200px width will render at viewport width, so all text in it must be legible at ~360px container width.

If step 4 shows the hero's terminal text becoming unreadable on mobile, add a media-query-equivalent within the SVG `<style>` block that enlarges text below a `vmin` threshold — or accept it (README readers on mobile are a minority).

## Maintenance

- Pipeline stage added → duplicate one pill element in `pipeline.svg`, adjust the gradient stops, update the two prose paragraphs under "How it works."
- Version bumps → update the version shields.io URL in README (one line).
- New feature tile → add a fifth column to the `<table>` and a new `tile-*.svg`.
- Palette change → regex find-replace token hex across the 7 SVGs.

No tooling required. All edits are text edits.

## Out of scope (deferred)

- **Animated GIF demo.** Considered but cut: recording, optimizing, and maintaining a GIF has real overhead, and the static hero already carries the pitch.
- **Azure service icons (official pack).** Considered but cut: licensing + attribution overhead, and risks misleading visitors about Microsoft affiliation.
- **GitHub Pages mini-site.** Considered but cut per user's scope reduction.
- **Dual light/dark SVG variants.** GitHub supports `<picture>` with `prefers-color-scheme` media queries for images, but authoring two variants doubles maintenance cost for marginal gain. Tuned-for-dark is sufficient.

## Risks

- **SVG text rendering inconsistencies** across browsers/platforms. Mitigated by using only system font stacks and avoiding fancy typography features.
- **GitHub's camo image proxy** occasionally caches stale versions of SVGs. If a visual update doesn't appear, it resolves on its own within hours or with a cache-busting query string.
- **Hero width on mobile.** 1200px SVG at 360px viewport means text is ~0.3x size. Acceptable in practice (fonts stay above 8–10 physical px) but flag-worthy if readability complaints arise.
