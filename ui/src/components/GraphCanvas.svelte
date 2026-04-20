<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import cytoscape from "cytoscape";
  import elk from "cytoscape-elk";
  import nodeHtmlLabel from "cytoscape-node-html-label";
  import { nodes, edges, selectedNodeKey, fitSignal, layoutSignal } from "../lib/store";
  import type { Node as GNode, Edge as GEdge, NodeKind } from "../lib/types";
  import { cidrToRange, cidrContains } from "../lib/cidr";

  cytoscape.use(elk);
  nodeHtmlLabel(cytoscape as any);

  let container: HTMLDivElement;
  let cy: cytoscape.Core | null = null;

  function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  /** Context bucket for a NodeKind — drives background gradient. */
  function contextOf(kind: string): string {
    if (kind.includes("dns")) return "dns";
    return "network";
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
      commandId: string | null;
      parent?: string;      // compound RG parent id
      kind: NodeKind;
      name: string;
      origin: string;
      status: string;
      cidr?: string;
      range?: string;
      extraProps?: Array<[string, string]>;  // non-cidr props, e.g. [["sku","Basic"], ["gateway-type","Vpn"]]
      height?: number;
      width?: number;
    };
    classes?: string;
  }

  /** Collect all non-cidr props as display-friendly (key, value) pairs. */
  function otherProps(n: GNode): Array<[string, string]> {
    const out: Array<[string, string]> = [];
    const p = n.props ?? {};
    for (const [k, v] of Object.entries(p)) {
      if (k === "cidr") continue;
      if (typeof v === "string") out.push([k, v]);
      else if (typeof v === "boolean") out.push([k, v ? "yes" : "no"]);
      else if (Array.isArray(v)) out.push([k, v.filter(x => typeof x === "string").join(", ")]);
    }
    return out;
  }

  function estimateHeight(data: { cidr?: string; range?: string; extraProps?: Array<[string, string]> }): number {
    // Base: padding (12) + pill row (~16) + pill margin (6) + name (~18) = 52. Each extra row adds ~14.
    const extra = data.extraProps ? Math.min(data.extraProps.length, 3) : 0;
    const rows = (data.cidr ? 1 : 0) + (data.range ? 1 : 0) + extra;
    return 52 + rows * 14;
  }

  function estimateWidth(data: { name: string; kind: string; cidr?: string; range?: string; extraProps?: Array<[string, string]> }): number {
    // Rough pixel widths for system-ui fonts at the sizes we use.
    const nameCharW = 7.5;   // 13px bold
    const cidrCharW = 6.8;   // 11px
    const rangeCharW = 6.2;  // 10px
    const propCharW = 6.0;   // 10px
    const pillCharW = 5.8;   // 9px + borders
    const padding = 22;      // left+right padding + safety

    const nameW = data.name.length * nameCharW;
    const pillW = data.kind.length * pillCharW + 18;  // pill has its own padding
    const cidrW = data.cidr ? data.cidr.length * cidrCharW + 30 : 0;  // +count suffix
    const rangeW = data.range ? data.range.length * rangeCharW : 0;
    const propW = Math.max(0, ...(data.extraProps ?? []).slice(0, 3).map(([k, v]) => {
      const truncated = v.length > 40 ? 40 : v.length;
      return (k.length + 2 + truncated) * propCharW;
    }));

    const contentW = Math.max(nameW, pillW, cidrW, rangeW, propW);
    return Math.max(170, Math.min(320, Math.ceil(contentW + padding)));
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

      const extraProps = otherProps(n);
      if (n.kind === "vnet" && prefixes.length > 1) {
        vnetPrefixesByKey[key] = prefixes;
        prefixes.forEach((p, i) => {
          const nodeData = {
            id: vnetVisualId(key, i),
            logicalKey: key,
            commandId: n.command_id ?? null,
            parent,
            kind: n.kind,
            name: n.name,
            origin: n.origin,
            status: n.status.kind,
            cidr: p,
            range: cidrToRange(p) ? `${cidrToRange(p)!.first} – ${cidrToRange(p)!.last}` : undefined,
            extraProps,
          };
          visualNodes.push({ data: { ...nodeData, height: estimateHeight(nodeData), width: estimateWidth(nodeData) }, classes: `ctx-${contextOf(n.kind)}` });
        });
      } else {
        const cidr = displayCidr(n);
        const nodeData = {
          id: key,
          logicalKey: key,
          commandId: n.command_id ?? null,
          parent,
          kind: n.kind,
          name: n.name,
          origin: n.origin,
          status: n.status.kind,
          cidr,
          range: cidr && cidrToRange(cidr) ? `${cidrToRange(cidr)!.first} – ${cidrToRange(cidr)!.last}` : undefined,
          extraProps,
        };
        visualNodes.push({ data: { ...nodeData, height: estimateHeight(nodeData), width: estimateWidth(nodeData) }, classes: `ctx-${contextOf(n.kind)}` });
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
    const range = data.range ?? "";
    const cidr = data.cidr ?? "";
    const countSuffix = (() => {
      if (!data.cidr) return "";
      const r = cidrToRange(data.cidr);
      return r ? ` (${r.count})` : "";
    })();
    const truncate = (s: string, max = 40) => s.length > max ? s.slice(0, max - 1) + "\u2026" : s;
    const extras = (data.extraProps ?? []).slice(0, 3)
      .map(([k, v]) => `<div class="azn-prop"><span class="azn-pk">${escapeHtml(k)}:</span> ${escapeHtml(truncate(v))}</div>`)
      .join("");
    return `
      <div class="azn">
        <span class="azn-pill" data-k="${escapeHtml(data.kind)}">${escapeHtml(data.kind)}</span>
        <div class="azn-name">${escapeHtml(data.name)}</div>
        ${cidr ? `<div class="azn-cidr">${escapeHtml(cidr)}${countSuffix}</div>` : ""}
        ${range ? `<div class="azn-range">${escapeHtml(range)}</div>` : ""}
        ${extras}
      </div>`;
  }

  function escapeHtml(s: string): string {
    return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;" }[c]!));
  }

  onMount(() => {
    cy = cytoscape({
      container,
      wheelSensitivity: 1.0,
      style: [
        {
          selector: "node[kind]",
          style: {
            "shape": "round-rectangle",
            "background-color": "#ffffff",
            "border-color": "#4a90e2",
            "border-width": 1.5,
            "border-style": "dashed",
            "width": "data(width)",
            "height": "data(height)",
            "label": "",
            "shadow-blur": 6,
            "shadow-color": "#0b2447",
            "shadow-opacity": 0.15,
            "shadow-offset-x": 0,
            "shadow-offset-y": 2,
          } as any,
        },
        {
          selector: "node.ctx-network",
          style: {
            "background-fill": "linear-gradient",
            "background-gradient-stop-colors": "#f0f7ff #cfe3fb",
            "background-gradient-stop-positions": "0 100",
            "background-gradient-direction": "to-bottom-right",
          } as any,
        },
        {
          selector: "node.ctx-dns",
          style: {
            "background-fill": "linear-gradient",
            "background-gradient-stop-colors": "#faf5ff #e9d5ff",
            "background-gradient-stop-positions": "0 100",
            "background-gradient-direction": "to-bottom-right",
          } as any,
        },
        { selector: "node[origin = 'Ghost']", style: { "border-color": "#888", "border-style": "dashed" } as any },
        { selector: "node[status = 'running']", style: { "border-color": "#b58022" } },
        { selector: "node[status = 'succeeded']", style: { "border-color": "#2a8f3d" } },
        { selector: "node[status = 'failed']", style: { "border-color": "#b53030" } },
        { selector: "node[status = 'missing']", style: { "border-color": "#ff8c1a", "border-style": "dashed" } as any },
        { selector: "node[status = 'exists']",  style: { "border-color": "#2a8f3d" } as any },
        { selector: "node[status = 'verifying']", style: { "border-color": "#b58022" } as any },
        { selector: "node.selected",
          style: {
            "border-width": 3,
            "border-color": "#0b2447",
          } as any },
        {
          selector: "node.rg",
          style: {
            "shape": "round-rectangle",
            "background-fill": "linear-gradient",
            "background-gradient-stop-colors": "#fafcff #eef5ff",
            "background-gradient-stop-positions": "0 100",
            "background-gradient-direction": "to-bottom",
            "border-color": "#4a90e2",
            "border-width": 1.5,
            "border-style": "dashed",
            "label": "data(label)",
            "text-halign": "center",
            "text-valign": "top",
            "text-margin-y": -4,
            "color": "#4a90e2",
            "font-size": 12,
            "font-weight": 700,
            "text-background-color": "#ffffff",
            "text-background-opacity": 1,
            "text-background-padding": "6px",
            "padding": "18px",
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
            "taxi-turn": "50%",
          } as any,
        },
      ],
    });

    // Register HTML label extension BEFORE any nodes are added, so the 'add'
    // event-driven attachment fires for every node including the initial batch.
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

    cy.add(buildElements($nodes, $edges) as any);
    cy.layout({
      name: "elk",
      elk: {
        "algorithm": "layered",
        "elk.direction": "DOWN",
        "elk.layered.spacing.nodeNodeBetweenLayers": 40,
        "elk.spacing.nodeNode": 20,
        "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
        "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
        "elk.edgeRouting": "ORTHOGONAL",
        "elk.layered.mergeEdges": "true",
        "elk.layered.unnecessaryBendpoints": "true",
      },
      nodeDimensionsIncludeLabels: false,
    } as any).run();

    cy.on("tap", "node[kind]", (ev) => {
      const logical = ev.target.data("logicalKey") as string;
      selectedNodeKey.set(logical);
    });

  });

  $: if (cy) {
    cy.elements().remove();
    cy.add(buildElements($nodes, $edges) as any);
    cy.layout({
      name: "elk",
      elk: {
        "algorithm": "layered",
        "elk.direction": "DOWN",
        "elk.layered.spacing.nodeNodeBetweenLayers": 40,
        "elk.spacing.nodeNode": 20,
        "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
        "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
        "elk.edgeRouting": "ORTHOGONAL",
        "elk.layered.mergeEdges": "true",
        "elk.layered.unnecessaryBendpoints": "true",
      },
      nodeDimensionsIncludeLabels: false,
    } as any).run();
  }

  // Apply .selected class to all visual nodes sharing the selected logical key.
  // If the selected node is off-screen, animate-pan the viewport to center it.
  $: if (cy) {
    cy.$("node.selected").removeClass("selected");
    const key = $selectedNodeKey;
    if (key) {
      const sel = cy.nodes(`[logicalKey = "${key}"]`);
      sel.addClass("selected");
      if (sel.length > 0) {
        const first = sel.first();
        const bb = first.renderedBoundingBox();
        const inView =
          bb.x1 >= 0 && bb.x2 <= cy.width() &&
          bb.y1 >= 0 && bb.y2 <= cy.height();
        if (!inView) {
          cy.animate({ center: { eles: first }, duration: 300 });
        }
      }
    }
  }

  // Fit-to-screen signal from Toolbar.
  $: if (cy && $fitSignal > 0) {
    cy.fit(undefined, 30);
  }

  // Re-layout signal from Toolbar.
  $: if (cy && $layoutSignal > 0) {
    cy.layout({
      name: "elk",
      elk: {
        "algorithm": "layered",
        "elk.direction": "DOWN",
        "elk.layered.spacing.nodeNodeBetweenLayers": 40,
        "elk.spacing.nodeNode": 20,
        "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
        "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
        "elk.edgeRouting": "ORTHOGONAL",
        "elk.layered.mergeEdges": "true",
        "elk.layered.unnecessaryBendpoints": "true",
      },
      nodeDimensionsIncludeLabels: false,
    } as any).run();
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
    width: 100%;
    box-sizing: border-box;
    padding: 6px 10px;
    line-height: 1.3;
    display: flex;
    flex-direction: column;
    text-align: center;
  }
  :global(.azn-pill) {
    align-self: flex-start;
    margin-bottom: 6px;
    font-size: 9px; font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    background: #f3f4f6; color: #374151;
    border: 1px solid #9ca3af;
    text-transform: lowercase;
    letter-spacing: .04em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    box-shadow: 0 1px 2px rgba(11, 36, 71, 0.15);
  }
  :global(.azn-pill[data-k="vnet"])          { background:#e0f2fe; color:#0369a1; border-color:#0ea5e9; }
  :global(.azn-pill[data-k="subnet"])        { background:#dcfce7; color:#15803d; border-color:#22c55e; }
  :global(.azn-pill[data-k="nsg"])           { background:#fef3c7; color:#92400e; border-color:#f59e0b; }
  :global(.azn-pill[data-k="nsg-rule"])      { background:#ffedd5; color:#9a3412; border-color:#f97316; }
  :global(.azn-pill[data-k="public-ip"])     { background:#cffafe; color:#0e7490; border-color:#06b6d4; }
  :global(.azn-pill[data-k="nic"])           { background:#f3e8ff; color:#6b21a8; border-color:#a855f7; }
  :global(.azn-pill[data-k="lb"])            { background:#fce7f3; color:#9d174d; border-color:#ec4899; }
  :global(.azn-pill[data-k="route-table"])   { background:#fef9c3; color:#854d0e; border-color:#eab308; }
  :global(.azn-pill[data-k="vnet-gateway"])  { background:#e0e7ff; color:#3730a3; border-color:#6366f1; }
  :global(.azn-pill[data-k="local-gateway"]) { background:#ccfbf1; color:#115e59; border-color:#14b8a6; }
  :global(.azn-pill[data-k="vpn-connection"]){ background:#ffe4e6; color:#9f1239; border-color:#f43f5e; }
  :global(.azn-pill[data-k="vnet-peering"])  { background:#ecfccb; color:#3f6212; border-color:#84cc16; }
  :global(.azn-pill[data-k="dns-resolver"])  { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }
  :global(.azn-pill[data-k="private-dns-zone"]) { background:#f5f3ff; color:#4c1d95; border-color:#7c3aed; }
  :global(.azn-pill[data-k="private-dns-link"]) { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }
  :global(.azn-name) { font-weight: 700; font-size: 13px; color: #0b2447; letter-spacing: -0.01em; text-align: center; word-break: break-all; }
  :global(.azn-cidr) { color: #c9184a; font-size: 11px; font-variant-numeric: tabular-nums; margin-top: 2px; }
  :global(.azn-range) { color: #444; font-size: 10px; font-variant-numeric: tabular-nums; }
  :global(.azn-prop) { color: #555; font-size: 10px; margin-top: 1px; }
  :global(.azn-pk) { color: #888; }
</style>
