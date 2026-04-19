# Graph Node Visual Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current single-line rounded-rectangle node rendering with a rich multi-line visual (icon + name + CIDR + IP range), nested inside a resource-group container frame, using a top-down tree layout with orthogonal edges. Multi-prefix VNets render as N visual nodes per prefix.

**Architecture:** Three layers. (1) Rust parser extracts CIDR into `node.props["cidr"]` driven by a new `props` field in the argmap. (2) A frontend CIDR helper computes IP ranges and containment. (3) `GraphCanvas.svelte` swaps to `cytoscape-node-html-label`, builds compound RG parents, expands multi-prefix VNets into `#pN` visual nodes, retargets subnet→VNet edge sources by CIDR containment, and switches to TB dagre with `taxi` edges.

**Tech Stack:** Rust (serde, regex-free parsing), TypeScript, Svelte 4, Cytoscape 3 + dagre + `cytoscape-node-html-label`, vitest for frontend unit tests.

---

## Phase 1: Backend — argmap props + parser

### Task 1: Add `props` field to ArgMapEntry

**Files:**
- Modify: `src-tauri/src/parser/argmap.rs`

- [ ] **Step 1: Add the struct field**

Replace the `ArgMapEntry` struct (lines 24–29) in `src-tauri/src/parser/argmap.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgMapEntry {
    pub produces: Produces,
    #[serde(default)] pub scope: ScopeFlags,
    #[serde(default)] pub refs: Vec<RefSpec>,
    /// Map of prop-name → CLI flag. Parser reads these and populates `node.props`.
    /// Example: `{ "cidr": "--address-prefixes" }`.
    #[serde(default)] pub props: std::collections::HashMap<String, String>,
}
```

- [ ] **Step 2: Run existing tests to verify no regression**

```bash
cd src-tauri && cargo test --lib parser::argmap 2>&1 | tail -10
```

Expected: all existing argmap tests still pass; `props` defaults to empty.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/parser/argmap.rs
git commit -m "parser(argmap): add optional props field for declared flag → prop extraction"
```

---

### Task 2: Parser writes `node.props` from argmap props

**Files:**
- Modify: `src-tauri/src/parser/parse.rs`

- [ ] **Step 1: Write failing test for single-valued prop**

Append to the `tests` mod in `src-tauri/src/parser/parse.rs` (after the existing tests):

```rust
    #[test]
    fn subnet_create_populates_cidr_prop() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefixes 10.0.0.0/27",
            &m, &g,
        ).unwrap();
        let subnet = p.new_nodes.iter().find(|n| n.kind == NodeKind::Subnet).unwrap();
        let cidr = subnet.props.get("cidr").expect("cidr prop missing");
        assert_eq!(cidr, &serde_json::json!("10.0.0.0/27"));
    }
```

- [ ] **Step 2: Run and verify FAIL**

```bash
cd src-tauri && cargo test --lib parser::parse::tests::subnet_create_populates_cidr_prop 2>&1 | tail -15
```

Expected: FAIL with `cidr prop missing` (or similar — the argmap hasn't declared the prop yet, and the parser doesn't read props at all).

- [ ] **Step 3: Implement `extract_flag_multi` + prop population**

Add this helper immediately after `extract_flag` (which ends near line 60 of `src-tauri/src/parser/parse.rs`):

```rust
/// Like `extract_flag` but collects all consecutive non-flag tokens after the flag.
/// For `--address-prefixes 10.0.0.0/26 10.0.1.0/26 --some-other-flag x`, returns
/// `vec!["10.0.0.0/26", "10.0.1.0/26"]`.
fn extract_flag_multi<'a>(rest: &'a [String], flag: &str) -> Vec<&'a str> {
    let short = short_alias(flag);
    let mut out = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let t = &rest[i];
        let hit = t == flag || short.is_some_and(|s| t == s);
        if hit {
            i += 1;
            while i < rest.len() && !rest[i].starts_with('-') {
                out.push(rest[i].as_str());
                i += 1;
            }
            return out;
        }
        if let Some(v) = t.strip_prefix(&format!("{flag}=")) {
            out.push(v);
            return out;
        }
        i += 1;
    }
    out
}
```

Then, inside `pub fn parse(...)`, after the `new_nodes.push(produces_node);` line (near the end of the function), the `produces_node` was already moved. We need to populate props **before** the push. Replace the block from `let command_id = format!("cmd-{}", ...);` through `let produces_id = produces_node.id.clone();` with:

```rust
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());
    let mut produces_node = Node::declared(kind, name.clone(), scope.clone(), command_id.clone());
    let produces_id = produces_node.id.clone();

    // Populate declared props from argmap's `props` map.
    for (prop_name, flag) in &entry.props {
        let vals = extract_flag_multi(rest, flag);
        match vals.len() {
            0 => {} // flag not present — leave prop unset
            1 => {
                produces_node.props.insert(prop_name.clone(), serde_json::Value::String(vals[0].to_string()));
            }
            _ => {
                let arr = vals.iter().map(|s| serde_json::Value::String(s.to_string())).collect();
                produces_node.props.insert(prop_name.clone(), serde_json::Value::Array(arr));
            }
        }
    }
```

- [ ] **Step 4: Run test to verify PASS**

```bash
cd src-tauri && cargo test --lib parser::parse::tests::subnet_create_populates_cidr_prop 2>&1 | tail -10
```

Expected: FAIL — the argmap doesn't yet declare the `cidr` prop for subnet. That's the next task. Leave the failing test in place.

---

### Task 3: Declare CIDR prop for VNet/Subnet/Public IP in arg-map.json

**Files:**
- Modify: `src-tauri/arg-map.json`

- [ ] **Step 1: Add props declarations**

Replace `src-tauri/arg-map.json` in full:

```json
{
  "network vnet create": {
    "produces": { "kind": "vnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     [],
    "props":    { "cidr": "--address-prefixes" }
  },
  "network vnet subnet create": {
    "produces": { "kind": "subnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "vnet", "via": "--vnet-name", "required": true }
    ],
    "props":    { "cidr": "--address-prefixes" }
  },
  "network nsg create": {
    "produces": { "kind": "nsg", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  },
  "network nsg rule create": {
    "produces": { "kind": "nsg-rule", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "nsg", "via": "--nsg-name", "required": true }
    ]
  },
  "network public-ip create": {
    "produces": { "kind": "public-ip", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     [],
    "props":    { "cidr": "--address" }
  },
  "network nic create": {
    "produces": { "kind": "nic", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs": [
      { "kind": "subnet",    "via": "--subnet",             "required": true },
      { "kind": "public-ip", "via": "--public-ip-address",  "required": false },
      { "kind": "nsg",       "via": "--network-security-group", "required": false }
    ]
  },
  "network route-table create": {
    "produces": { "kind": "route-table", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  }
}
```

- [ ] **Step 2: Run the Task-2 test — now it should PASS**

```bash
cd src-tauri && cargo test --lib parser::parse::tests::subnet_create_populates_cidr_prop 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 3: Add multi-valued prop test**

Append to the `tests` mod in `src-tauri/src/parser/parse.rs`:

```rust
    #[test]
    fn vnet_create_populates_multi_prefix_cidr() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network vnet create --name net-hub --resource-group rg --address-prefixes 10.0.0.0/26 10.0.1.0/26",
            &m, &g,
        ).unwrap();
        let vnet = p.new_nodes.iter().find(|n| n.kind == NodeKind::Vnet).unwrap();
        let cidr = vnet.props.get("cidr").expect("cidr prop missing");
        assert_eq!(cidr, &serde_json::json!(["10.0.0.0/26", "10.0.1.0/26"]));
    }

    #[test]
    fn nsg_create_without_cidr_has_no_cidr_prop() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network nsg create --name n --resource-group rg",
            &m, &g,
        ).unwrap();
        let nsg = &p.new_nodes[0];
        assert!(nsg.props.get("cidr").is_none());
    }
```

- [ ] **Step 4: Run all parser tests**

```bash
cd src-tauri && cargo test --lib parser 2>&1 | tail -10
```

Expected: all pass, including the two new cases.

- [ ] **Step 5: Run full test suite to confirm no regressions**

```bash
cd src-tauri && cargo test 2>&1 | tail -15
```

Expected: all tests pass (existing parser tests + new ones + argmap_bundle integration test).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/parser/parse.rs src-tauri/arg-map.json
git commit -m "parser: extract CIDR into node.props for vnet/subnet/public-ip"
```

Hook will auto-bump patch version.

---

## Phase 2: Frontend — CIDR helper

### Task 4: Install vitest

**Files:**
- Modify: `ui/package.json`

- [ ] **Step 1: Add vitest as dev dep**

```bash
cd ui && npm install --save-dev vitest@^1.6.0
```

- [ ] **Step 2: Add test script to ui/package.json**

Edit `ui/package.json`, change the `"scripts"` block (currently has `dev`, `build`, `preview`) to include a `test` script:

```json
  "scripts": {
    "dev": "vite --port 1420 --strictPort",
    "build": "svelte-check && vite build",
    "preview": "vite preview",
    "test": "vitest run"
  },
```

- [ ] **Step 3: Verify vitest runs (no tests yet, should report zero)**

```bash
cd ui && npm test 2>&1 | tail -5
```

Expected: vitest starts, reports `No test files found, exiting with code 1` (or similar — that's fine; we just verified the binary works).

- [ ] **Step 4: Commit**

```bash
git add ui/package.json ui/package-lock.json
git commit -m "ui: add vitest dev dep for unit tests"
```

---

### Task 5: CIDR helper with TDD

**Files:**
- Create: `ui/src/lib/cidr.ts`
- Create: `ui/src/lib/cidr.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `ui/src/lib/cidr.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { parseCidr, cidrToRange, cidrContains } from "./cidr";

describe("parseCidr", () => {
  it("parses a standard /26", () => {
    const r = parseCidr("10.0.0.0/26");
    expect(r).toEqual({ base: 0x0A000000, prefixLen: 26 });
  });
  it("returns null for IPv6", () => {
    expect(parseCidr("::1/128")).toBeNull();
  });
  it("returns null for malformed input", () => {
    expect(parseCidr("not-a-cidr")).toBeNull();
    expect(parseCidr("10.0.0.0")).toBeNull();
    expect(parseCidr("10.0.0.0/33")).toBeNull();
    expect(parseCidr("10.0.0.1/24")).toBeNull(); // host bits set
  });
  it("accepts /0 and /32", () => {
    expect(parseCidr("0.0.0.0/0")).toEqual({ base: 0, prefixLen: 0 });
    expect(parseCidr("192.168.1.1/32")).toEqual({ base: 0xC0A80101, prefixLen: 32 });
  });
});

describe("cidrToRange", () => {
  it("computes /26 range and count", () => {
    expect(cidrToRange("10.0.0.0/26")).toEqual({
      first: "10.0.0.0", last: "10.0.0.63", count: 64,
    });
  });
  it("handles /32 as single IP", () => {
    expect(cidrToRange("192.168.1.1/32")).toEqual({
      first: "192.168.1.1", last: "192.168.1.1", count: 1,
    });
  });
  it("handles /0 as full space", () => {
    const r = cidrToRange("0.0.0.0/0");
    expect(r?.first).toBe("0.0.0.0");
    expect(r?.last).toBe("255.255.255.255");
    expect(r?.count).toBe(4294967296);
  });
  it("returns null for IPv6 or malformed", () => {
    expect(cidrToRange("::1/128")).toBeNull();
    expect(cidrToRange("bogus")).toBeNull();
  });
});

describe("cidrContains", () => {
  it("outer contains inner (subnet in vnet)", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.0.0/27")).toBe(true);
    expect(cidrContains("10.0.0.0/26", "10.0.0.32/27")).toBe(true);
  });
  it("outer does not contain unrelated inner", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.1.0/27")).toBe(false);
  });
  it("outer contains itself", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.0.0/26")).toBe(true);
  });
  it("inner smaller prefix cannot fit in outer larger prefix", () => {
    expect(cidrContains("10.0.0.0/27", "10.0.0.0/26")).toBe(false);
  });
  it("returns false for IPv6 inputs", () => {
    expect(cidrContains("::/0", "::1/128")).toBe(false);
    expect(cidrContains("10.0.0.0/24", "::1/128")).toBe(false);
  });
  it("returns false for missing/malformed input", () => {
    expect(cidrContains("10.0.0.0/24", undefined)).toBe(false);
    expect(cidrContains("10.0.0.0/24", "")).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify FAIL**

```bash
cd ui && npm test 2>&1 | tail -15
```

Expected: all tests fail with import error (`cidr.ts` doesn't exist).

- [ ] **Step 3: Implement `cidr.ts`**

Create `ui/src/lib/cidr.ts`:

```ts
export interface ParsedCidr { base: number; prefixLen: number }
export interface CidrRange { first: string; last: string; count: number }

const IPV4_OCTET = /^(25[0-5]|2[0-4]\d|1?\d?\d)$/;

function parseIpv4(s: string): number | null {
  const parts = s.split(".");
  if (parts.length !== 4) return null;
  let n = 0;
  for (const p of parts) {
    if (!IPV4_OCTET.test(p)) return null;
    n = (n * 256 + Number(p)) >>> 0;
  }
  return n;
}

function ipv4ToString(n: number): string {
  return [(n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff].join(".");
}

export function parseCidr(s: string): ParsedCidr | null {
  if (typeof s !== "string" || s.includes(":")) return null;
  const slash = s.indexOf("/");
  if (slash < 0) return null;
  const ipPart = s.slice(0, slash);
  const lenPart = s.slice(slash + 1);
  if (!/^\d+$/.test(lenPart)) return null;
  const prefixLen = Number(lenPart);
  if (prefixLen < 0 || prefixLen > 32) return null;
  const base = parseIpv4(ipPart);
  if (base === null) return null;
  const mask = prefixLen === 0 ? 0 : (0xffffffff << (32 - prefixLen)) >>> 0;
  if ((base & mask) !== base) return null; // host bits set
  return { base, prefixLen };
}

export function cidrToRange(s: string): CidrRange | null {
  const p = parseCidr(s);
  if (!p) return null;
  const size = 2 ** (32 - p.prefixLen);
  const last = (p.base + size - 1) >>> 0;
  return { first: ipv4ToString(p.base), last: ipv4ToString(last), count: size };
}

export function cidrContains(outer: string, inner: string | undefined): boolean {
  if (!inner) return false;
  const o = parseCidr(outer);
  if (!o) return false;
  const i = parseCidr(inner);
  if (!i) return false;
  if (i.prefixLen < o.prefixLen) return false;
  const mask = o.prefixLen === 0 ? 0 : (0xffffffff << (32 - o.prefixLen)) >>> 0;
  return (i.base & mask) === o.base;
}
```

- [ ] **Step 4: Run tests to verify PASS**

```bash
cd ui && npm test 2>&1 | tail -15
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add ui/src/lib/cidr.ts ui/src/lib/cidr.test.ts
git commit -m "ui(cidr): pure IPv4 helpers — parse, range, contains, with vitest coverage"
```

---

## Phase 3: Frontend — icons module

### Task 6: Per-kind SVG icons as data URLs

**Files:**
- Create: `ui/src/lib/icons.ts`

- [ ] **Step 1: Create the icons module**

Create `ui/src/lib/icons.ts`:

```ts
import type { NodeKind } from "./types";

const COLOR = "#4a90e2";
const SW = 1.5;

function svgDataUrl(inner: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="14" height="14" ` +
    `fill="none" stroke="${COLOR}" stroke-width="${SW}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export const KIND_ICONS: Record<NodeKind, string> = {
  // Diamond with 3 dots (VNet "⟨⋯⟩" vibe)
  "vnet": svgDataUrl(`<path d="M8 1.5 L14.5 8 L8 14.5 L1.5 8 Z"/><circle cx="5.5" cy="8" r="0.7" fill="${COLOR}" stroke="none"/><circle cx="8" cy="8" r="0.7" fill="${COLOR}" stroke="none"/><circle cx="10.5" cy="8" r="0.7" fill="${COLOR}" stroke="none"/>`),
  // 2x2 grid (Subnet)
  "subnet": svgDataUrl(`<rect x="2" y="2" width="5" height="5" rx="0.5"/><rect x="9" y="2" width="5" height="5" rx="0.5"/><rect x="2" y="9" width="5" height="5" rx="0.5"/><rect x="9" y="9" width="5" height="5" rx="0.5"/>`),
  // Heraldic shield (NSG)
  "nsg": svgDataUrl(`<path d="M8 1.5 L14 3.5 V8 Q14 12.5 8 14.5 Q2 12.5 2 8 V3.5 Z"/>`),
  // Shield + arrow (NSG rule)
  "nsg-rule": svgDataUrl(`<path d="M8 1.5 L13.5 3.5 V8 Q13.5 12 8 14 Q2.5 12 2.5 8 V3.5 Z"/><path d="M6 8 L10 8 M8.5 6.5 L10 8 L8.5 9.5"/>`),
  // Circle + outward arrow (Public IP)
  "public-ip": svgDataUrl(`<circle cx="6" cy="10" r="3.5"/><path d="M9 7 L14 2 M11 2 H14 V5"/>`),
  // Ethernet plug (NIC)
  "nic": svgDataUrl(`<rect x="4" y="3" width="8" height="9" rx="1"/><rect x="6" y="12" width="4" height="2.5"/><path d="M6 6 V9 M8 6 V9 M10 6 V9"/>`),
  // Splitting arrows (Load balancer)
  "lb": svgDataUrl(`<path d="M8 2 V8 M8 8 L3 14 M8 8 L13 14"/><path d="M1.5 12 L3 14 L4.5 12 M11.5 12 L13 14 L14.5 12"/>`),
  // Forked arrow (Route table)
  "route-table": svgDataUrl(`<path d="M8 2 V6 M8 6 L3 11 V14 M8 6 L13 11 V14"/><path d="M1.5 12 L3 14 L4.5 12 M11.5 12 L13 14 L14.5 12"/>`),
  // Folder (Resource group)
  "rg": svgDataUrl(`<path d="M1.5 4 H6 L7.5 5.5 H14.5 V13 Q14.5 14 13.5 14 H2.5 Q1.5 14 1.5 13 Z"/>`),
};
```

- [ ] **Step 2: Sanity-check the module loads (no tests, quick smoke)**

```bash
cd ui && node --input-type=module -e "import('./src/lib/icons.ts').catch(e => { console.error(e.message); process.exit(1); })" 2>&1 | head -3
```

If the above fails due to TS-loading issues, skip this smoke step — vite/svelte will resolve imports at build time. The module will be validated when `GraphCanvas.svelte` imports it in Task 8.

- [ ] **Step 3: Commit**

```bash
git add ui/src/lib/icons.ts
git commit -m "ui(icons): per-kind monochrome SVG glyphs as data URLs"
```

---

## Phase 4: Frontend — GraphCanvas rewrite

### Task 7: Install `cytoscape-node-html-label`

**Files:**
- Modify: `ui/package.json`

- [ ] **Step 1: Install the extension**

```bash
cd ui && npm install cytoscape-node-html-label@^2.0.0
```

- [ ] **Step 2: Commit**

```bash
git add ui/package.json ui/package-lock.json
git commit -m "ui: add cytoscape-node-html-label for rich node rendering"
```

---

### Task 8: Rewrite GraphCanvas with HTML nodes, compound RGs, multi-prefix expansion, TB layout

**Files:**
- Modify: `ui/src/components/GraphCanvas.svelte`

- [ ] **Step 1: Replace the entire file**

Replace `ui/src/components/GraphCanvas.svelte` in full:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import cytoscape from "cytoscape";
  import dagre from "cytoscape-dagre";
  import nodeHtmlLabel from "cytoscape-node-html-label";
  import { nodes, edges, selectedNodeKey } from "../lib/store";
  import type { Node as GNode, Edge as GEdge, NodeKind } from "../lib/types";
  import { cidrToRange, cidrContains } from "../lib/cidr";
  import { KIND_ICONS } from "../lib/icons";

  cytoscape.use(dagre);
  nodeHtmlLabel(cytoscape as any);

  let container: HTMLDivElement;
  let cy: cytoscape.Core | null = null;

  function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  function rgId(rg: string): string {
    return `rg:${rg}`;
  }

  /** Returns the list of prefixes for a VNet node, or empty array if none/not applicable. */
  function vnetPrefixes(n: GNode): string[] {
    if (n.kind !== "vnet") return [];
    const raw = n.props?.cidr as unknown;
    if (typeof raw === "string") return [raw];
    if (Array.isArray(raw)) return raw.filter((x): x is string => typeof x === "string");
    return [];
  }

  /** Visual node id for a VNet prefix. */
  function vnetVisualId(logicalKey: string, prefixIdx: number): string {
    return `${logicalKey}#p${prefixIdx}`;
  }

  /** Pick a single CIDR to display on a node (first prefix for multi-prefix visual nodes). */
  function displayCidr(n: GNode, prefixOverride?: string): string | undefined {
    if (prefixOverride) return prefixOverride;
    const raw = n.props?.cidr as unknown;
    if (typeof raw === "string") return raw;
    if (Array.isArray(raw) && typeof raw[0] === "string") return raw[0];
    return undefined;
  }

  interface VisualNode {
    data: {
      id: string;           // visual id (may include #pN)
      logicalKey: string;   // backend key (for selection)
      parent?: string;      // compound RG parent id
      kind: NodeKind;
      name: string;
      origin: string;
      status: string;
      cidr?: string;
      range?: string;
    };
  }

  interface VisualEdge {
    data: { id: string; source: string; target: string; via: string };
  }

  function buildElements(ns: GNode[], es: GEdge[]): (VisualNode | VisualEdge | any)[] {
    // 1. Collect unique RGs → compound parent nodes
    const rgs = new Set<string>();
    for (const n of ns) rgs.add(n.scope.resource_group);
    const rgNodes = Array.from(rgs).map((rg) => ({
      data: { id: rgId(rg), label: rg },
      classes: "rg",
    }));

    // 2. For each logical node, emit 1..N visual nodes
    const visualNodes: VisualNode[] = [];
    const vnetPrefixesByKey: Record<string, string[]> = {};
    for (const n of ns) {
      const key = keyOf(n.id);
      const parent = rgId(n.scope.resource_group);
      const prefixes = vnetPrefixes(n);

      if (n.kind === "vnet" && prefixes.length > 1) {
        vnetPrefixesByKey[key] = prefixes;
        prefixes.forEach((p, i) => {
          visualNodes.push({
            data: {
              id: vnetVisualId(key, i),
              logicalKey: key,
              parent,
              kind: n.kind,
              name: n.name,
              origin: n.origin,
              status: n.status.kind,
              cidr: p,
              range: cidrToRange(p) ? `${cidrToRange(p)!.first} – ${cidrToRange(p)!.last}` : undefined,
            },
          });
        });
      } else {
        const cidr = displayCidr(n);
        visualNodes.push({
          data: {
            id: key,
            logicalKey: key,
            parent,
            kind: n.kind,
            name: n.name,
            origin: n.origin,
            status: n.status.kind,
            cidr,
            range: cidr && cidrToRange(cidr) ? `${cidrToRange(cidr)!.first} – ${cidrToRange(cidr)!.last}` : undefined,
          },
        });
      }
    }

    // 3. For each edge, retarget source if VNet source has multiple prefixes
    const nodesByKey: Record<string, GNode> = {};
    for (const n of ns) nodesByKey[keyOf(n.id)] = n;

    const visualEdges: VisualEdge[] = es.map((e, i) => {
      const fromKey = keyOf(e.from);
      const toKey = keyOf(e.to);
      let source = fromKey;
      const prefixes = vnetPrefixesByKey[fromKey];
      if (prefixes && prefixes.length > 1) {
        const subnet = nodesByKey[toKey];
        const rawSub = subnet?.props?.cidr as unknown;
        const subnetCidr = typeof rawSub === "string" ? rawSub : Array.isArray(rawSub) ? rawSub[0] : undefined;
        let idx = 0;
        if (typeof subnetCidr === "string") {
          const found = prefixes.findIndex((p) => cidrContains(p, subnetCidr));
          if (found >= 0) idx = found;
        }
        source = vnetVisualId(fromKey, idx);
      }
      return { data: { id: `e${i}`, source, target: toKey, via: e.via } };
    });

    return [...rgNodes, ...visualNodes, ...visualEdges];
  }

  function nodeHtmlTemplate(data: VisualNode["data"]): string {
    const icon = KIND_ICONS[data.kind];
    const range = data.range ?? "";
    const cidr = data.cidr ?? "";
    const countSuffix = (() => {
      if (!data.cidr) return "";
      const r = cidrToRange(data.cidr);
      return r ? ` (${r.count})` : "";
    })();
    return `
      <div class="azn">
        <div class="azn-head">
          <img class="azn-icon" src="${icon}" alt="" />
          <span class="azn-name">${escapeHtml(data.name)}</span>
        </div>
        ${cidr ? `<div class="azn-cidr">${escapeHtml(cidr)}${countSuffix}</div>` : ""}
        ${range ? `<div class="azn-range">${escapeHtml(range)}</div>` : ""}
      </div>`;
  }

  function escapeHtml(s: string): string {
    return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" }[c]!));
  }

  onMount(() => {
    cy = cytoscape({
      container,
      elements: buildElements($nodes, $edges),
      layout: { name: "dagre", rankDir: "TB", nodeSep: 40, rankSep: 60 } as any,
      wheelSensitivity: 0.2,
      style: [
        {
          selector: "node[kind]",
          style: {
            "shape": "round-rectangle",
            "background-color": "#ffffff",
            "border-color": "#4a90e2",
            "border-width": 1.5,
            "width": 180,
            "height": 70,
            "label": "",
          } as any,
        },
        { selector: "node[origin = 'Ghost']", style: { "border-color": "#888", "border-style": "dashed" } as any },
        { selector: "node[status = 'running']", style: { "border-color": "#b58022" } },
        { selector: "node[status = 'succeeded']", style: { "border-color": "#2a8f3d" } },
        { selector: "node[status = 'failed']", style: { "border-color": "#b53030" } },
        {
          selector: "node.rg",
          style: {
            "shape": "round-rectangle",
            "background-color": "#fafcff",
            "border-color": "#4a90e2",
            "border-width": 1.5,
            "label": "data(label)",
            "text-halign": "left",
            "text-valign": "top",
            "text-margin-y": -8,
            "text-margin-x": 14,
            "color": "#4a90e2",
            "font-size": 11,
            "font-weight": 600,
            "padding": "14px",
          } as any,
        },
        {
          selector: "edge",
          style: {
            "width": 1.5,
            "line-color": "#4a90e2",
            "target-arrow-color": "#4a90e2",
            "target-arrow-shape": "triangle",
            "curve-style": "taxi",
            "taxi-direction": "vertical",
          } as any,
        },
      ],
    });

    (cy as any).nodeHtmlLabel([
      {
        query: "node[kind]",
        halign: "center",
        valign: "center",
        halignBox: "center",
        valignBox: "center",
        tpl: nodeHtmlTemplate,
      },
    ]);

    cy.on("tap", "node[kind]", (ev) => {
      const logical = ev.target.data("logicalKey") as string;
      selectedNodeKey.set(logical);
    });
  });

  $: if (cy) {
    cy.elements().remove();
    cy.add(buildElements($nodes, $edges) as any);
    cy.layout({ name: "dagre", rankDir: "TB", nodeSep: 40, rankSep: 60 } as any).run();
  }

  onDestroy(() => {
    cy?.destroy();
    cy = null;
  });
</script>

<div bind:this={container} class="canvas"></div>

<style>
  .canvas { width: 100%; height: 100%; background: #fff; }

  :global(.azn) {
    font-family: system-ui, sans-serif;
    padding: 6px 10px;
    min-width: 140px;
    max-width: 220px;
    text-align: left;
    line-height: 1.3;
  }
  :global(.azn-head) { display: flex; align-items: center; gap: 6px; }
  :global(.azn-icon) { width: 14px; height: 14px; flex-shrink: 0; }
  :global(.azn-name) { font-weight: 700; font-size: 11px; color: #0b2447; }
  :global(.azn-cidr) { color: #c9184a; font-size: 10px; font-variant-numeric: tabular-nums; margin-top: 2px; }
  :global(.azn-range) { color: #666; font-size: 9px; font-variant-numeric: tabular-nums; }
</style>
```

- [ ] **Step 2: Build the UI to catch type errors**

```bash
cd ui && npm run build 2>&1 | tail -20
```

Expected: `svelte-check` passes with 0 errors, `vite build` succeeds, and `dist/` is produced. Warnings are acceptable. If errors appear, read them — the most common is a type mismatch in `buildElements`'s return type.

- [ ] **Step 3: Run Rust test suite to confirm backend still green**

```bash
cd src-tauri && cargo test 2>&1 | tail -6
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/GraphCanvas.svelte
git commit -m "ui(graph): rich HTML nodes, RG compound frame, TB tree, multi-prefix VNet expansion"
```

---

## Phase 5: Verification

### Task 9: Manual smoke test via Tauri dev

**Files:** none (runtime verification only)

- [ ] **Step 1: Launch dev app**

```bash
cd src-tauri && cargo tauri dev 2>&1
```

Wait for Vite to report listening on port 1420 and for Tauri to open the desktop window. (If `cargo tauri dev` times out or hangs in this shell, run it in a real terminal.)

- [ ] **Step 2: Enter the reference sketch's commands**

In the desktop app's CommandPane, add these one at a time:

```
az network vnet create -g lakeflow-mssql -n net-hub --address-prefixes 10.0.0.0/26 10.0.1.0/26
az network vnet subnet create -g lakeflow-mssql -n snet-app --vnet-name net-hub --address-prefixes 10.0.0.0/27
az network vnet subnet create -g lakeflow-mssql -n GatewaySubnet --vnet-name net-hub --address-prefixes 10.0.0.32/27
az network vnet subnet create -g lakeflow-mssql -n dns-resolver-in --vnet-name net-hub --address-prefixes 10.0.1.0/28
az network vnet subnet create -g lakeflow-mssql -n dns-resolver-out --vnet-name net-hub --address-prefixes 10.0.1.16/28
```

- [ ] **Step 3: Verify visual acceptance checklist**

- [ ] Outer frame labeled `lakeflow-mssql` surrounds all nodes (top-left label on a rounded rect).
- [ ] Two VNet prefix nodes visible: one showing `10.0.0.0/26 (64)` with range `10.0.0.0 – 10.0.0.63`; one showing `10.0.1.0/26 (64)` with range `10.0.1.0 – 10.0.1.63`.
- [ ] Both VNet nodes show the diamond-with-dots icon and the name `net-hub` (bold).
- [ ] `snet-app` and `GatewaySubnet` attach under the `10.0.0.0/26` prefix node (not the `10.0.1.0/26`).
- [ ] `dns-resolver-in` and `dns-resolver-out` attach under the `10.0.1.0/26` prefix node.
- [ ] All subnet nodes show the 2×2-grid icon, their CIDR in red, and their IP range in dim text.
- [ ] Edges are orthogonal (right-angle), flowing from top (VNets) to bottom (subnets), with arrowheads at the subnet end.
- [ ] Clicking any prefix node of a multi-prefix VNet populates the DetailPane with exactly one VNet (not two).

- [ ] **Step 4: Kill dev server and commit any leftover tweaks**

If the verification revealed a bug, fix inline, rebuild, re-verify. Then:

```bash
git status --short
# only commit-worthy tweaks; if clean, skip
```

---

## Spec Coverage Check

| Spec Section | Implementation Task |
|---|---|
| 2. Goals — icon + name + CIDR + IP range | Task 8 (HTML template), Task 6 (icons), Task 5 (range), Task 3 (cidr data) |
| 2. Goals — RG labeled frame | Task 8 (compound node + `.rg` style) |
| 2. Goals — TB tree layout | Task 8 (`rankDir: "TB"`) |
| 2. Goals — orthogonal edges | Task 8 (`curve-style: "taxi"`) |
| 2. Goals — multi-prefix VNet = N visual nodes | Task 8 (`buildElements` VNet expansion) |
| 2. Goals — no Azure brand assets | Task 6 (generic SVG glyphs) |
| 3. Non-goals (explicit exclusions) | None needed — not implemented |
| 4. Decisions — `cytoscape-node-html-label` | Task 7 (install), Task 8 (register) |
| 4. Decisions — argmap `props` drives `node.props` | Task 1 (struct), Task 2 (parser), Task 3 (json) |
| 4. Decisions — frontend CIDR helpers | Task 5 |
| 4. Decisions — visual `#pN` suffixes | Task 8 (`vnetVisualId`) |
| 4. Decisions — compound RG parent id `rg:<name>` | Task 8 (`rgId`) |
| 4. Decisions — selection uses logical key | Task 8 (`on("tap", ...)` reads `logicalKey`) |
| 8.2 Source retargeting | Task 8 (`buildElements` edge loop) |
| 9. Icons (per-kind glyphs) | Task 6 |
| 10. CIDR helper specification | Task 5 (full test coverage) |
| 11. Backend verification tests | Task 2, Task 3 |
| 11. Frontend unit tests | Task 5 |
| 11. Manual smoke test | Task 9 |

---

## Execution Options

Plan complete and saved to `docs/superpowers/plans/2026-04-19-graph-node-visual-plan.md`.

Two ways to execute:

**1. Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks.

**2. Inline Execution** — All tasks in this session with checkpoints.

Which approach?
