# Graph Node Visual Redesign — Design Spec

**Date:** 2026-04-19
**Status:** Design approved; implementation to follow.

## 1. Problem

The current `GraphCanvas` renders each resource as a single-line rounded rectangle showing only `kind · name`. Operators need to see network topology at a glance — resource group boundaries, CIDR allocations, IP ranges, and resource kinds — without clicking into a detail pane.

The target visual comes from a user-supplied sketch of an Azure network with a resource group frame, VNet-and-subnet trees, CIDR annotations, IP ranges, and per-kind icons.

## 2. Goals (v1)

Render the graph such that:

- Each node shows **icon + name + CIDR(s) + IP range** where applicable.
- **Resource group** appears as a labeled bounding frame around its resources.
- Layout is **top-down tree** (parents above children).
- Edges are **orthogonal** (diagram-style, not bezier curves).
- A **VNet with N address prefixes renders as N visual nodes**, one per prefix. Subnet-to-VNet edges are retargeted to the prefix whose CIDR contains the subnet's CIDR.
- No Azure brand assets — icons are generic SVG glyphs.

## 3. Non-goals (v1)

- Per-node status color redesign (keeps existing declared/ghost/running/succeeded/failed palette).
- Detail lines beyond name for kinds with no CIDR (NSG, NIC, LB, Route Table, NSG Rule).
- Collapse/expand for the RG container.
- Editing CIDRs via the UI.
- IPv6 CIDRs — any `::`-bearing CIDR suppresses the IP-range line (name + CIDR still shown).
- Multi-subscription layout (one subscription per canvas, as today).

## 4. Decisions (locked)

| Dimension | Decision |
|---|---|
| Rendering engine | Cytoscape (unchanged) + `cytoscape-node-html-label` extension |
| Layout | dagre, `rankDir: "TB"`, compound nodes enabled |
| Edge style | `curve-style: "taxi"` (orthogonal right-angle routing) |
| CIDR source | Argmap entry declares which flag carries the CIDR; parser populates `Node.props["cidr"]` |
| CIDR storage | `Node.props["cidr"]` = string (single prefix) OR array of strings (multi-prefix) |
| IP-range computation | Frontend-only helper (`ui/src/lib/cidr.ts`), pure function |
| Multi-prefix VNet rendering | UI-layer expansion to N visual nodes, IDs suffixed `#p0`, `#p1`, …; backend unchanged |
| Resource-group container | One Cytoscape compound parent node per unique `scope.resource_group`, id `rg:<name>` |
| Selection target | Clicking a `#pN` visual node selects the underlying VNet (all prefix copies share selection & status) |
| Icon source | Hand-drawn SVG data URLs, monochrome navy `#4a90e2`, 14px |

## 5. Architecture

### 5.1 Data flow

```
az command
  │
  ▼
[Rust parser]──reads argmap.props──populates node.props["cidr"]
  │
  ▼
Graph state (snapshot) ──ipc──▶ [Svelte store] ──▶ [GraphCanvas.svelte]
                                                         │
                                                         ▼
                                          UI expansion: VNet × N prefixes → N visual nodes
                                                         │
                                                         ▼
                                       Edge retargeting: subnet→vnet edges pick prefix by CIDR containment
                                                         │
                                                         ▼
                                           Cytoscape elements (with compound RG parents)
                                                         │
                                                         ▼
                                         cytoscape-node-html-label renders each node as HTML
```

### 5.2 Components and responsibilities

**Backend (Rust):**

| File | Role |
|---|---|
| `src-tauri/src/parser/argmap.rs` | Extend `ArgMapEntry` with `props: HashMap<String, String>` (prop-name → flag-name). |
| `src-tauri/src/parser/parse.rs` | For each declared node, iterate `entry.props` and call `extract_flag`; store non-null values in `node.props` as JSON scalars or arrays. Multi-valued flags (`--address-prefixes a b c`) produce an array. |
| `src-tauri/arg-map.json` | Add `"props": { "cidr": "--address-prefixes" }` (VNet, Subnet) and `"cidr": "--address"` (Public IP, if applicable). |

**Frontend (TypeScript/Svelte):**

| File | Role |
|---|---|
| `ui/src/lib/cidr.ts` | Pure helpers: `parseCidr(s) → { base, prefixLen }`; `cidrToRange(s) → { first, last, count }`; `cidrContains(outer, inner) → boolean`; all IPv4 only. IPv6 returns `null` where applicable. |
| `ui/src/lib/icons.ts` | Per-`NodeKind` SVG data URL map. |
| `ui/src/components/GraphCanvas.svelte` | (1) Import `cytoscape-node-html-label` and register. (2) Build Cytoscape elements with compound RG parents, prefix-expanded VNets, and retargeted edges. (3) Define the HTML template per node. (4) Switch layout to TB dagre with orthogonal edges. |
| `ui/package.json` | Add `cytoscape-node-html-label` dep. |

## 6. Node rendering (HTML template)

Each Cytoscape node's HTML body:

```
┌─────────────────────────────────┐
│ [icon]  name-in-bold            │   ← row 1
│ 10.0.0.0/26 (64)                │   ← row 2, red, tabular-nums, only if CIDR present
│ 10.0.0.0 – 10.0.0.63            │   ← row 3, dim, tabular-nums, only if IPv4 CIDR
└─────────────────────────────────┘
```

- Corner radius: 8px. Width: auto (min ~140px, max ~220px, text wraps).
- Border: 1.5px, color by origin (`#4a90e2` declared, `#888` ghost dashed).
- Background: `#ffffff` (was `#eaf3ff` for declared — switch to white for contrast against RG frame).
- Font: system-ui, 11px name (bold), 10px CIDR (red `#c9184a`), 9px range (dim `#666`).

## 7. Resource-group container

For each unique `scope.resource_group` in the graph:

- Create a compound parent node in Cytoscape: `{ data: { id: "rg:<name>", label: "<name>" }, classes: "rg" }`.
- All non-parent nodes in that RG receive `parent: "rg:<name>"` in their Cytoscape data.
- Style:
  - `shape: "round-rectangle"`, `border: 1.5px #4a90e2`, `background: #fafcff`, `corner-radius: 10px`.
  - Label top-left via `text-halign: left, text-valign: top`, `text-margin-y: -10, text-margin-x: 14`, `background: #fafcff` on the label to cut through the top border.
- dagre auto-sizes the frame to contain its children.

## 8. Multi-prefix VNet expansion

### 8.1 Rule

Backend sends one VNet node with `props.cidr` being a single string OR an array. Frontend expansion:

```ts
function expandVnet(node: GNode): GNode[] {
  const cidr = node.props?.cidr;
  if (!Array.isArray(cidr) || cidr.length <= 1) return [node]; // single node
  return cidr.map((p, i) => ({
    ...node,
    _logicalId: keyOf(node.id),           // for selection lookup
    _displayId: `${keyOf(node.id)}#p${i}`,
    props: { ...node.props, cidr: p },    // single prefix per visual node
  }));
}
```

### 8.2 Source retargeting

Edges point from VNet (source) → subnet (target) in the current graph model. When the source VNet has been expanded into N visual prefix nodes, the edge's **source** gets rewritten from the logical vnet id to whichever `#pN` prefix's CIDR contains the subnet's CIDR. The edge target (the subnet's id) is unchanged.

Subnet-side CIDR resolution: if the subnet's `props.cidr` is an array (rare for subnets, but possible), use the first element. If the subnet is a ghost (no props at all), if containment cannot be resolved, or if either CIDR is IPv6/malformed, fall back to `#p0`.

```ts
function retargetSource(e: GEdge, vnetPrefixes: Record<string, string[]>, subnetById: Record<string, GNode>): GEdge {
  const vnetKey = keyOf(e.from);
  const prefixes = vnetPrefixes[vnetKey];
  if (!prefixes || prefixes.length <= 1) return e;

  const subnet = subnetById[keyOf(e.to)];
  const raw = subnet?.props?.cidr;
  const subnetCidr = Array.isArray(raw) ? raw[0] : raw;

  let idx = 0;
  if (typeof subnetCidr === "string") {
    const found = prefixes.findIndex(p => cidrContains(p, subnetCidr));
    if (found >= 0) idx = found;
  }
  return { ...e, _sourceDisplayId: `${vnetKey}#p${idx}` };
}
```

### 8.3 Selection & status

- Clicking any `#pN` visual node: GraphCanvas dispatches selection with the **logical** node id (the backend id), so `DetailPane` shows one VNet.
- Status-derived styles (running/succeeded/failed) apply to all `#pN` siblings uniformly because they read from the logical node's status.

## 9. Icons

Seven SVG glyphs + one for RG. All ~14×14 px, stroke `#4a90e2`, stroke-width 1.5, fill `none`.

| Kind | Glyph |
|---|---|
| VNet | Diamond outline with 3 dots inside (`⟨⋯⟩` vibe) |
| Subnet | 2×2 grid of 3px squares |
| NSG | Heraldic shield |
| NSG Rule | Shield with a small arrow |
| Public IP | Circle + outward arrow |
| NIC | Ethernet plug silhouette |
| LB | Two arrows splitting from a central point |
| Route Table | Y-fork with arrowheads |
| Resource Group | Folder tab (shown as RG compound-node label prefix) |

All stored as base64 data URLs in `ui/src/lib/icons.ts` keyed by `NodeKind`.

## 10. CIDR helper specification

`ui/src/lib/cidr.ts` exports pure, side-effect-free functions. IPv4 only.

### `parseCidr(s: string): { base: number; prefixLen: number } | null`
Parses `"10.0.0.0/26"` into `{ base: 0x0A000000, prefixLen: 26 }`. Returns `null` on malformed input, IPv6, or host bits outside the prefix.

### `cidrToRange(s: string): { first: string; last: string; count: number } | null`
Returns human-readable range. For `10.0.0.0/26` → `{ first: "10.0.0.0", last: "10.0.0.63", count: 64 }`. For `/32`, count is 1 and first == last. For `/0`, count is `2^32` (expressed as number; safe in JS).

### `cidrContains(outer: string, inner: string | undefined): boolean`
True iff outer is a valid CIDR, inner is a valid CIDR (or IP), and inner's IP range fits entirely within outer's. Returns `false` for any IPv6 input or malformed input.

### Tests
Co-located in `ui/src/lib/cidr.test.ts`, run with **vitest** (added as a dev dep). Vitest pairs naturally with the project's existing Vite build and requires no further config.

## 11. Verification

### Backend

- Existing parser tests continue to pass.
- New test: `vnet_create_populates_cidr_prop` — parses `az network vnet create --name v --resource-group rg --address-prefixes 10.0.0.0/26 10.0.1.0/26`, asserts `node.props["cidr"]` is a JSON array with both strings.
- New test: `subnet_create_populates_cidr_prop` — parses subnet with `--address-prefixes 10.0.0.0/27`.
- New test: `missing_cidr_prop_is_fine` — parses NSG create (no CIDR), asserts `node.props` has no `"cidr"` key.

### Frontend

- Unit tests on `cidr.ts` covering: `/32`, `/0`, `/24`, host-bit violation (invalid), containment positive/negative, IPv6 rejected, malformed input returns `null`.
- Manual smoke test:
  1. Start app with sample commands:
     ```
     az network vnet create -g lakeflow-mssql -n net-hub --address-prefixes 10.0.0.0/26 10.0.1.0/26
     az network vnet subnet create -g lakeflow-mssql -n snet-app --vnet-name net-hub --address-prefixes 10.0.0.0/27
     az network vnet subnet create -g lakeflow-mssql -n GatewaySubnet --vnet-name net-hub --address-prefixes 10.0.0.32/27
     az network vnet subnet create -g lakeflow-mssql -n dns-resolver-in --vnet-name net-hub --address-prefixes 10.0.1.0/28
     ```
  2. Verify: RG frame labeled `lakeflow-mssql`; two `net-hub` prefix nodes; `snet-app` & `GatewaySubnet` attach under the `/26` node; `dns-resolver-in` attaches under the `/26 (10.0.1)` node; all nodes show name + CIDR + range.

## 12. Success criteria

- Reference-sketch layout reproducible from a set of ~5 commands.
- Multi-prefix VNet renders as multiple prefix nodes, each with correctly-routed subnet edges.
- Resource group appears as a single labeled frame around all its resources.
- Selecting any `#pN` visual node shows one VNet in `DetailPane` (not N).
- Ghost VNets render with dashed border and no CIDR line.
- Existing graph behavior (add command, dry-run, live execute) unchanged.
