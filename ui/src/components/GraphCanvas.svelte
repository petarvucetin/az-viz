<script lang="ts">
  import { SvelteFlow, SvelteFlowProvider, MarkerType, type Node as SFNode, type Edge as SFEdge } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { writable } from "svelte/store";
  import { nodes as storeNodes, edges as storeEdges, selectedNodeKey, fitSignal, layoutSignal } from "../lib/store";
  import type { Node as GNode, Edge as GEdge } from "../lib/types";
  import { cidrToRange, cidrContains } from "../lib/cidr";
  import { runLayout } from "../lib/layout";
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

  /** Strip the #pN suffix to get the logical key. */
  function logicalOf(nodeId: string): string {
    const i = nodeId.lastIndexOf("#p");
    return i >= 0 ? nodeId.slice(0, i) : nodeId;
  }

  // ─── Node / Edge types ──────────────────────────────────────────────────────

  const nodeTypes = {
    resource: ResourceNode as any,
    rg: ResourceGroupNode as any,
  };

  // ─── Svelte Flow stores ─────────────────────────────────────────────────────

  const sfNodes = writable<SFNode[]>([]);
  const sfEdges = writable<SFEdge[]>([]);

  // Generation counter prevents stale async layout results from overwriting
  // results from a newer build triggered by a rapid store update.
  let layoutGen = 0;

  // ─── Build + layout ─────────────────────────────────────────────────────────

  function buildElements(ns: GNode[], es: GEdge[], selKey: string | null): { nodes: SFNode[]; edges: SFEdge[] } {
    // 1. RG compound nodes (must be added before their children)
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

    // 2. Visual nodes (multi-prefix VNet expansion)
    const vnetPrefixesByKey: Record<string, string[]> = {};
    const nodesByKey: Record<string, GNode> = {};
    for (const n of ns) nodesByKey[keyOf(n.id)] = n;

    for (const n of ns) {
      const key = keyOf(n.id);
      const parent = rgId(n.scope.resource_group);
      const prefixes = vnetPrefixes(n);
      const extraProps = otherProps(n);

      const mkData = (visualId: string, cidr: string | undefined) => ({
        logicalKey: key,
        kind: n.kind,
        name: n.name,
        origin: n.origin,
        status: n.status.kind,
        cidr,
        range: cidr && cidrToRange(cidr) ? `${cidrToRange(cidr)!.first} – ${cidrToRange(cidr)!.last}` : undefined,
        extraProps,
        context: contextOf(n.kind),
        selectedDirect: selKey !== null && key === selKey,
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
            extent: "parent",
            width: w,
            height: h,
            draggable: false,
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
          extent: "parent",
          width: w,
          height: h,
          draggable: false,
          selectable: false,
        });
      }
    }

    // 3. Edges — retarget source when VNet has multiple prefixes (VNet→Subnet direction)
    const resultEdges: SFEdge[] = es.map((e, i) => {
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
      const isSelected = selKey !== null && (logicalOf(source) === selKey || logicalOf(toKey) === selKey);
      const edgeColor = isSelected ? "#0b2447" : "#4a90e2";
      return {
        id: `e${i}`,
        source,
        target: toKey,
        type: "smoothstep",
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

    // Bail if a newer layout superseded us.
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

    sfNodes.set(positioned);
    sfEdges.set(rawEdges);
  }

  // Rebuild + layout whenever nodes, edges, or selection changes.
  $: buildAndLayout($storeNodes, $storeEdges, $selectedNodeKey);

  // ─── Node click ─────────────────────────────────────────────────────────────

  function onNodeClick(event: CustomEvent<{ node: SFNode; event: MouseEvent | TouchEvent }>) {
    const clicked = event.detail?.node;
    if (!clicked) return;
    const logical = (clicked.data as any)?.logicalKey as string | undefined;
    if (logical) selectedNodeKey.set(logical);
  }

  // ─── Re-layout signal ────────────────────────────────────────────────────────

  $: if ($layoutSignal > 0) {
    buildAndLayout($storeNodes, $storeEdges, $selectedNodeKey);
  }
</script>

<SvelteFlowProvider>
  <div class="canvas">
    <SvelteFlow
      {nodeTypes}
      nodes={sfNodes}
      edges={sfEdges}
      fitView
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={false}
      deleteKey={null}
      defaultEdgeOptions={{ type: "smoothstep", style: "stroke:#4a90e2;stroke-width:1.5;", markerEnd: { type: MarkerType.ArrowClosed, color: "#4a90e2" } }}
      on:nodeclick={onNodeClick}
    >
      <FlowActions {fitSignal} />
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
</style>
