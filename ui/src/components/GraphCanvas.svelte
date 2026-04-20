<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import cytoscape from "cytoscape";
  import dagre from "cytoscape-dagre";
  import nodeHtmlLabel from "cytoscape-node-html-label";
  import { nodes, edges, selectedNodeKey, fitSignal } from "../lib/store";
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
      commandId: string | null;
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
              commandId: n.command_id ?? null,
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
            commandId: n.command_id ?? null,
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
            "width": 190,
            "height": 68,
            "label": "",
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
            "background-color": "#fafcff",
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
            "text-background-color": "#fafcff",
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
    cy.layout({ name: "dagre", rankDir: "TB", nodeSep: 40, rankSep: 60 } as any).run();

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

  // Apply .selected class to all visual nodes sharing the selected logical key.
  $: if (cy) {
    cy.$("node.selected").removeClass("selected");
    const key = $selectedNodeKey;
    if (key) {
      cy.nodes(`[logicalKey = "${key}"]`).addClass("selected");
    }
  }

  // Fit-to-screen signal from Toolbar.
  $: if (cy && $fitSignal > 0) {
    cy.fit(undefined, 30);
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
    min-width: 150px;
    max-width: 210px;
    text-align: center;
    line-height: 1.3;
  }
  :global(.azn-head) { display: flex; align-items: center; justify-content: center; gap: 6px; }
  :global(.azn-icon) { width: 14px; height: 14px; flex-shrink: 0; }
  :global(.azn-name) { font-weight: 700; font-size: 12px; color: #0b2447; }
  :global(.azn-cidr) { color: #c9184a; font-size: 11px; font-variant-numeric: tabular-nums; margin-top: 2px; }
  :global(.azn-range) { color: #444; font-size: 10px; font-variant-numeric: tabular-nums; }
</style>
