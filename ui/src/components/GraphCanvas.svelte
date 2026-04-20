<script lang="ts">
  import { SvelteFlow, SvelteFlowProvider, MarkerType, type Node as SFNode, type Edge as SFEdge } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { untrack } from "svelte";
  import { appState } from "../lib/store.svelte";
  import type { Node as GNode, Edge as GEdge } from "../lib/types";
  import { cidrToRange, cidrContains } from "../lib/cidr";
  import { runLayout } from "../lib/layout";
  import { computeBlocked } from "../lib/blocking";
  import ResourceNode from "./ResourceNode.svelte";
  import ResourceGroupNode from "./ResourceGroupNode.svelte";
  import FlowActions from "./FlowActions.svelte";

  // ─── Helpers ────────────────────────────────────────────────────────────────

  function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }
  function rgId(rg: string): string { return `rg:${rg}`; }
  function contextOf(kind: string): string { return kind.includes("dns") ? "dns" : "network"; }
  function vnetVisualId(logicalKey: string, prefixIdx: number): string { return `${logicalKey}#p${prefixIdx}`; }

  function vnetPrefixes(n: GNode): string[] {
    if (n.kind !== "vnet") return [];
    const raw = n.props?.cidr as unknown;
    if (typeof raw === "string") return [raw];
    if (Array.isArray(raw)) return raw.filter((x): x is string => typeof x === "string");
    return [];
  }
  function displayCidr(n: GNode): string | undefined {
    const raw = n.props?.cidr as unknown;
    if (typeof raw === "string") return raw;
    if (Array.isArray(raw) && typeof raw[0] === "string") return raw[0];
    return undefined;
  }
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
  function estimateWidth(data: { name: string; kind: string; cidr?: string; range?: string; extraProps?: Array<[string, string]> }): number {
    const nameCharW = 7.5, cidrCharW = 6.8, rangeCharW = 6.2, propCharW = 6.0, pillCharW = 5.8;
    const padding = 22;
    const nameW = data.name.length * nameCharW;
    const pillW = data.kind.length * pillCharW + 18;
    const cidrW = data.cidr ? data.cidr.length * cidrCharW + 30 : 0;
    const rangeW = data.range ? data.range.length * rangeCharW : 0;
    const propW = Math.max(0, ...(data.extraProps ?? []).slice(0, 3).map(([k, v]) => (k.length + 2 + Math.min(v.length, 40)) * propCharW));
    const contentW = Math.max(nameW, pillW, cidrW, rangeW, propW);
    return Math.max(170, Math.min(320, Math.ceil(contentW + padding)));
  }
  function estimateHeight(data: { cidr?: string; range?: string; extraProps?: Array<[string, string]> }): number {
    const extra = data.extraProps ? Math.min(data.extraProps.length, 3) : 0;
    const rows = (data.cidr ? 1 : 0) + (data.range ? 1 : 0) + extra;
    return 52 + rows * 14;
  }

  function logicalOf(nodeId: string): string {
    const i = nodeId.lastIndexOf("#p");
    return i >= 0 ? nodeId.slice(0, i) : nodeId;
  }

  // ─── Node / Edge types ──────────────────────────────────────────────────────

  const nodeTypes = {
    resource: ResourceNode as any,
    rg: ResourceGroupNode as any,
  };

  // ─── Local state ────────────────────────────────────────────────────────────

  let sfNodes = $state<SFNode[]>([]);
  let sfEdges = $state<SFEdge[]>([]);
  let layoutGen = 0;

  // ─── Build + layout ─────────────────────────────────────────────────────────

  function buildElements(ns: GNode[], es: GEdge[], selKey: string | null): { nodes: SFNode[]; edges: SFEdge[] } {
    const blocked = computeBlocked(ns, es);
    const rgs = new Set<string>();
    for (const n of ns) rgs.add(n.scope.resource_group);

    const resultNodes: SFNode[] = [];
    for (const rg of rgs) {
      resultNodes.push({
        id: rgId(rg),
        type: "rg",
        position: { x: 0, y: 0 },
        data: { label: rg },
        draggable: false,
        selectable: false,
      });
    }

    const vnetPrefixesByKey: Record<string, string[]> = {};
    const nodesByKey: Record<string, GNode> = {};
    for (const n of ns) nodesByKey[keyOf(n.id)] = n;

    for (const n of ns) {
      const key = keyOf(n.id);
      const parent = rgId(n.scope.resource_group);
      const prefixes = vnetPrefixes(n);
      const extraP = otherProps(n);

      const mkData = (visualId: string, cidr: string | undefined) => ({
        logicalKey: key,
        kind: n.kind,
        name: n.name,
        origin: n.origin,
        status: n.status.kind,
        cidr,
        range: cidr && cidrToRange(cidr) ? `${cidrToRange(cidr)!.first} – ${cidrToRange(cidr)!.last}` : undefined,
        extraProps: extraP,
        context: contextOf(n.kind),
        selectedDirect: selKey !== null && key === selKey,
        blocked: blocked.has(key),
      });

      if (n.kind === "vnet" && prefixes.length > 1) {
        vnetPrefixesByKey[key] = prefixes;
        prefixes.forEach((p, i) => {
          const visualId = vnetVisualId(key, i);
          const d = mkData(visualId, p);
          const w = estimateWidth(d);
          const h = estimateHeight(d);
          resultNodes.push({
            id: visualId,
            type: "resource",
            position: { x: 0, y: 0 },
            data: d,
            parentId: parent,
            expandParent: true,
            width: w,
            height: h,
            selectable: false,
          });
        });
      } else {
        const cidr = displayCidr(n);
        const d = mkData(key, cidr);
        const w = estimateWidth(d);
        const h = estimateHeight(d);
        resultNodes.push({
          id: key,
          type: "resource",
          position: { x: 0, y: 0 },
          data: d,
          parentId: parent,
          expandParent: true,
          width: w,
          height: h,
          selectable: false,
        });
      }
    }

    const resultEdges: SFEdge[] = es.map((e) => {
      const fromKey = keyOf(e.from);
      const toKey = keyOf(e.to);
      let source = fromKey;
      const pfx = vnetPrefixesByKey[fromKey];
      if (pfx && pfx.length > 1) {
        const subnet = nodesByKey[toKey];
        const rawSub = subnet?.props?.cidr as unknown;
        const subnetCidr = typeof rawSub === "string" ? rawSub : Array.isArray(rawSub) ? rawSub[0] : undefined;
        let idx = 0;
        if (typeof subnetCidr === "string") {
          const found = pfx.findIndex((p) => cidrContains(p, subnetCidr));
          if (found >= 0) idx = found;
        }
        source = vnetVisualId(fromKey, idx);
      }
      const isSelected = selKey !== null && (logicalOf(source) === selKey || logicalOf(toKey) === selKey);
      const edgeColor = isSelected ? "#0b2447" : "#4a90e2";
      return {
        id: `${source}~${toKey}/${e.via}`,
        source,
        target: toKey,
        type: "default",
        style: isSelected
          ? "stroke:#0b2447;stroke-width:3;"
          : "stroke:#4a90e2;stroke-width:1.5;",
        zIndex: isSelected ? 10 : 0,
        markerEnd: { type: MarkerType.ArrowClosed, color: edgeColor },
      };
    });

    return { nodes: resultNodes, edges: resultEdges };
  }

  async function buildAndLayout(ns: GNode[], es: GEdge[], selKey: string | null) {
    const gen = ++layoutGen;
    const { nodes: rawNodes, edges: rawEdges } = buildElements(ns, es, selKey);

    const layoutNodes = rawNodes.map(n => ({
      id: n.id,
      width: (n.width as number) ?? 400,
      height: (n.height as number) ?? 200,
      parent: n.parentId,
    }));
    
    const layoutEdges = rawEdges.map(e => ({ id: e.id, source: e.source, target: e.target }));

    const { positions, sizes } = await runLayout(layoutNodes, layoutEdges);

    if (gen !== layoutGen) return;

    const positioned: SFNode[] = rawNodes.map(n => {
      const pos = positions[n.id] ?? { x: 0, y: 0 };
      const out: SFNode = { ...n, position: pos };
      if (n.type === "rg") {
        const sz = sizes[n.id];
        if (sz) { out.width = sz.width; out.height = sz.height; }
      }
      return out;
    });

    sfNodes = positioned;
    sfEdges = rawEdges;
  }

  // Rebuild + layout whenever the graph data changes.
  $effect(() => {
    const ns = appState.nodes;
    const es = appState.edges;
    buildAndLayout(ns, es, null);
  });

  // Apply selection-only updates without full ELK re-layout.
  $effect(() => {
    const selKey = appState.selectedNodeKey;
    untrack(() => {
      sfNodes = sfNodes.map(n => {
        if (n.type !== "resource") return n;
        const logical = (n.data as any)?.logicalKey;
        const sel = selKey !== null && logical === selKey;
        return { ...n, data: { ...(n.data as any), selectedDirect: sel } };
      });
      sfEdges = sfEdges.map(e => {
        const srcLogical = logicalOf(e.source);
        const tgtLogical = logicalOf(e.target);
        const on = selKey !== null && (srcLogical === selKey || tgtLogical === selKey);
        const color = on ? "#0b2447" : "#4a90e2";
        return {
          ...e,
          style: on ? "stroke:#0b2447;stroke-width:3;" : "stroke:#4a90e2;stroke-width:1.5;",
          zIndex: on ? 10 : 0,
          markerEnd: { type: MarkerType.ArrowClosed, color },
        };
      });
    });
  });

  // ─── Node click ─────────────────────────────────────────────────────────────

  function onNodeClick({ node }: { node: SFNode; event: MouseEvent | TouchEvent }) {
    const logical = (node.data as any)?.logicalKey as string | undefined;
    if (logical) appState.selectedNodeKey = logical;
  }

  // ─── Re-layout signal ────────────────────────────────────────────────────────
  let lastLayoutSignal = 0;
  $effect(() => {
    const v = appState.layoutSignal;
    if (v !== lastLayoutSignal && v > 0) {
      lastLayoutSignal = v;
      untrack(() => {
        buildAndLayout(appState.nodes, appState.edges, null);
      });
    }
  });
</script>

<SvelteFlowProvider>
  <div class="canvas">
    <SvelteFlow
      {nodeTypes}
      bind:nodes={sfNodes}
      bind:edges={sfEdges}
      fitView
      nodesDraggable={true}
      nodesConnectable={false}
      elementsSelectable={false}
      deleteKey={null}
      defaultEdgeOptions={{ type: "default", style: "stroke:#4a90e2;stroke-width:1.5;", markerEnd: { type: MarkerType.ArrowClosed, color: "#4a90e2" } }}
      onnodeclick={onNodeClick}
    >
      <FlowActions />
    </SvelteFlow>
  </div>
</SvelteFlowProvider>

<style>
  .canvas { width: 100%; height: 100%; background: #fff; }
  :global(.svelte-flow__background) { background: #fff; }
  :global(.svelte-flow__node) { border-radius: 0; background: transparent; border: none; padding: 0; }
  :global(.svelte-flow__node-rg) { pointer-events: none; }
  :global(.svelte-flow__attribution) { display: none; }
  :global(.svelte-flow__edges) { z-index: 5; }
  :global(.svelte-flow__edge-path) { stroke-width: 1.5; stroke: #4a90e2; fill: none; }
  :global(.svelte-flow__handle) { opacity: 0 !important; }
</style>
