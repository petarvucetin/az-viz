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
  import VariableNode from "./VariableNode.svelte";
  import GroupNode from "./GroupNode.svelte";
  import FlowActions from "./FlowActions.svelte";
  import FlowApiCapture from "./FlowApiCapture.svelte";
  import type { useSvelteFlow } from "@xyflow/svelte";

  let flowApi: ReturnType<typeof useSvelteFlow> | null = $state(null);

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
    variable: VariableNode as any,
    group: GroupNode as any,
  };

  // ─── Local state ────────────────────────────────────────────────────────────

  let sfNodes = $state<SFNode[]>([]);
  let sfEdges = $state<SFEdge[]>([]);
  let layoutGen = 0;

  // ─── Build + layout ─────────────────────────────────────────────────────────

  function buildElements(
    ns: GNode[],
    es: GEdge[],
    variables: import("../lib/types").Variable[],
    varConsumers: Record<string, string[]>,
    groups: import("../lib/types").Group[],
    groupNodes: Record<string, string[]>,
    selKey: string | null,
  ): { nodes: SFNode[]; edges: SFEdge[] } {
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

    // Group frame nodes. parentId = RG of the group's first member.
    // Build a nodeKey → groupId reverse map at the same time.
    const nodeKeyToGroupId: Record<string, string> = {};
    const groupById: Record<string, import("../lib/types").Group> = {};
    for (const gr of groups) groupById[gr.id] = gr;
    for (const [gid, keys] of Object.entries(groupNodes)) {
      for (const k of keys) nodeKeyToGroupId[k] = gid;
    }
    const nodesByKeyAll: Record<string, GNode> = {};
    for (const n of ns) nodesByKeyAll[keyOf(n.id)] = n;
    // Ordering so layered ELK places groups in declaration order (#1, #2, ...).
    const orderedGroupIds = groups.map(g => g.id).filter(gid => (groupNodes[gid]?.length ?? 0) > 0);
    for (const gid of orderedGroupIds) {
      const keys = groupNodes[gid] ?? [];
      const first = keys[0] ? nodesByKeyAll[keys[0]] : undefined;
      if (!first) continue;
      const rgParent = rgId(first.scope.resource_group);
      const gr = groupById[gid];
      resultNodes.push({
        id: `group-${gid}`,
        type: "group",
        position: { x: 0, y: 0 },
        data: { title: gr?.title ?? "(group)", logicalKey: `group:${gid}`,
                selectedDirect: selKey === `group:${gid}` },
        parentId: rgParent,
        expandParent: true,
        draggable: true,
        selectable: false,
      });
    }

    const vnetPrefixesByKey: Record<string, string[]> = {};
    const nodesByKey: Record<string, GNode> = {};
    for (const n of ns) nodesByKey[keyOf(n.id)] = n;

    for (const n of ns) {
      const key = keyOf(n.id);
      const gid = nodeKeyToGroupId[key];
      const parent = gid ? `group-${gid}` : rgId(n.scope.resource_group);
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
            draggable: true,
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
          // `expandParent: true` lets xyflow grow the parent on all four
          // sides during drag (it internally adjusts drag-start state when
          // shifting the parent origin). `extent` is intentionally omitted
          // because it snapshots parent bounds and doesn't re-read them,
          // which blocks continuous expansion mid-drag.
          expandParent: true,
          width: w,
          height: h,
          selectable: false,
          draggable: true,
        });
      }
    }

    // Variable nodes: attach each referenced variable to the same parent as
    // its first consumer — group if the consumer is grouped, RG otherwise.
    // Draw an edge from the consumer → variable so ELK places them
    // adjacently. A variable with multiple consumers renders once with
    // multiple incoming edges.
    const varEdgesFromConsumer: Array<{ consumerKey: string; varName: string }> = [];
    const varParent: Record<string, string> = {};
    for (const [consumerKey, names] of Object.entries(varConsumers)) {
      const consumer = nodesByKey[consumerKey];
      if (!consumer) continue;
      const consumerGroup = nodeKeyToGroupId[consumerKey];
      const parentId = consumerGroup
        ? `group-${consumerGroup}`
        : rgId(consumer.scope.resource_group);
      for (const name of names) {
        if (!(name in varParent)) varParent[name] = parentId;
        varEdgesFromConsumer.push({ consumerKey, varName: name });
      }
    }
    const varByName: Record<string, import("../lib/types").Variable> = {};
    for (const v of variables) varByName[v.name] = v;
    for (const [name, parent] of Object.entries(varParent)) {
      const v = varByName[name];
      if (!v) continue;
      const logicalKey = `var:${name}`;
      resultNodes.push({
        id: logicalKey,
        type: "variable",
        position: { x: 0, y: 0 },
        data: {
          name,
          resolved: v.resolved,
          origin: v.origin,
          logicalKey,
          selectedDirect: selKey === logicalKey,
        },
        parentId: parent,
        expandParent: true,
        width: Math.max(120, name.length * 8 + 30),
        height: 58,
        selectable: false,
        draggable: true,
      });
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

    // Edges consumer → variable. Use a subdued amber stroke so they read as
    // metadata wiring, not graph topology.
    for (const { consumerKey, varName } of varEdgesFromConsumer) {
      const target = `var:${varName}`;
      const on = selKey !== null && (consumerKey === selKey || target === selKey);
      resultEdges.push({
        id: `${consumerKey}~${target}`,
        source: consumerKey,
        target,
        type: "default",
        style: on
          ? "stroke:#9a3412;stroke-width:2.5;stroke-dasharray:4 3;"
          : "stroke:#fb923c;stroke-width:1.3;stroke-dasharray:4 3;",
        zIndex: on ? 10 : 0,
        markerEnd: { type: MarkerType.ArrowClosed, color: on ? "#9a3412" : "#fb923c" },
      });
    }

    return { nodes: resultNodes, edges: resultEdges };
  }

  async function buildAndLayout(
    ns: GNode[], es: GEdge[],
    variables: import("../lib/types").Variable[],
    varConsumers: Record<string, string[]>,
    groups: import("../lib/types").Group[],
    groupNodes: Record<string, string[]>,
    selKey: string | null,
  ) {
    const gen = ++layoutGen;
    const { nodes: rawNodes, edges: rawEdges } = buildElements(ns, es, variables, varConsumers, groups, groupNodes, selKey);

    const layoutNodes = rawNodes.map(n => ({
      id: n.id,
      width: (n.width as number) ?? 400,
      height: (n.height as number) ?? 200,
      parent: n.parentId,
    }));
    
    const layoutEdges = rawEdges.map(e => ({ id: e.id, source: e.source, target: e.target }));

    // Ordering edges (ELK-only, not rendered) pin consecutive groups into
    // declaration order so the layout shows #1 above #2 above #3 etc.
    const orderedGroupIds = groups.map(g => g.id).filter(gid => (groupNodes[gid]?.length ?? 0) > 0);
    for (let i = 0; i + 1 < orderedGroupIds.length; i++) {
      layoutEdges.push({
        id: `order-${orderedGroupIds[i]}-${orderedGroupIds[i + 1]}`,
        source: `group-${orderedGroupIds[i]}`,
        target: `group-${orderedGroupIds[i + 1]}`,
      });
    }

    const { positions, sizes } = await runLayout(layoutNodes, layoutEdges);

    if (gen !== layoutGen) return;

    const positioned: SFNode[] = rawNodes.map(n => {
      const pos = positions[n.id] ?? { x: 0, y: 0 };
      const out: SFNode = { ...n, position: pos };
      if (n.type === "rg" || n.type === "group") {
        const sz = sizes[n.id];
        if (sz) { out.width = sz.width; out.height = sz.height; }
      }
      return out;
    });

    sfNodes = positioned;
    sfEdges = rawEdges;
  }

  // Structural signature: set of node ids + parent links + edge src/tgt.
  // When it's unchanged, the snapshot only carries status/prop updates and
  // we patch node data in place — no re-layout, no viewport jump.
  function structuralSig(rawNodes: SFNode[], rawEdges: SFEdge[]): string {
    const ns = rawNodes
      .map(n => `${n.id}|${n.parentId ?? ""}|${n.type ?? ""}`)
      .sort()
      .join(",");
    const es = rawEdges.map(e => `${e.source}>${e.target}`).sort().join(",");
    return `${ns}||${es}`;
  }
  let lastSig = "";

  // Rebuild + layout whenever the graph data changes structurally; otherwise
  // patch the existing sfNodes in place so execution status changes (e.g.
  // succeeded → green border + ✓) don't disturb the viewport.
  $effect(() => {
    const ns = appState.nodes;
    const es = appState.edges;
    const vs = appState.variables;
    const vc = appState.varConsumers;
    const gs = appState.groups;
    const gn = appState.groupNodes;
    const { nodes: rawNodes, edges: rawEdges } = buildElements(ns, es, vs, vc, gs, gn, appState.selectedNodeKey);
    const sig = structuralSig(rawNodes, rawEdges);

    untrack(() => {
      if (sig === lastSig && sfNodes.length > 0) {
        const byId: Record<string, SFNode> = {};
        for (const n of rawNodes) byId[n.id] = n;
        sfNodes = sfNodes.map(existing => {
          const fresh = byId[existing.id];
          if (!fresh) return existing;
          return {
            ...existing,
            data: fresh.data,
          };
        });
        sfEdges = rawEdges;
        return;
      }
      lastSig = sig;
      buildAndLayout(ns, es, vs, vc, gs, gn, null);
    });
  });

  // Apply selection-only updates without full ELK re-layout.
  $effect(() => {
    const selKey = appState.selectedNodeKey;
    untrack(() => {
      sfNodes = sfNodes.map(n => {
        if (n.type !== "resource" && n.type !== "variable") return n;
        const logical = (n.data as any)?.logicalKey;
        const sel = selKey !== null && logical === selKey;
        return { ...n, data: { ...(n.data as any), selectedDirect: sel } };
      });
      sfEdges = sfEdges.map(e => {
        const isVarEdge = e.target.startsWith("var:");
        const srcLogical = logicalOf(e.source);
        const tgtLogical = e.target.startsWith("var:") ? e.target : logicalOf(e.target);
        const on = selKey !== null && (srcLogical === selKey || tgtLogical === selKey);
        if (isVarEdge) {
          const color = on ? "#9a3412" : "#fb923c";
          return {
            ...e,
            style: on
              ? "stroke:#9a3412;stroke-width:2.5;stroke-dasharray:4 3;"
              : "stroke:#fb923c;stroke-width:1.3;stroke-dasharray:4 3;",
            zIndex: on ? 10 : 0,
            markerEnd: { type: MarkerType.ArrowClosed, color },
          };
        }
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

  function onNodeClick(evt: { node: SFNode; event: MouseEvent | TouchEvent }) {
    // Click payload uses `node` (drag payload uses `targetNode` — different shapes).
    const logical = (evt.node?.data as any)?.logicalKey as string | undefined;
    if (logical) appState.selectedNodeKey = logical;
  }

  // ─── Dynamic parent resize on drag ─────────────────────────────────────────
  // Svelte Flow's `expandParent: true` grows a parent when a dragged child
  // crosses its edge but never shrinks it. During/after a drag we recompute
  // each ancestor's size from the bounding box of its direct children plus
  // padding, so the frame both expands when approached and contracts when
  // the child moves away.

  const FRAME_PAD = { left: 18, right: 18, top: 32, bottom: 18 };
  const MIN_FRAME_W = 180;
  const MIN_FRAME_H = 80;
  const EDGE_GROW_THRESHOLD = 40;
  const EDGE_GROW_STEP = 60;

  // Cascading right/bottom expansion up the ancestor chain. xyflow's
  // `expandParent: true` on each child handles the IMMEDIATE parent in all
  // four directions (including the tricky left/top cases where parent
  // origin must shift atomically with drag state). This handler picks up
  // the cascade above that — if a grown group now overflows its RG, we
  // grow the RG too. Left/top cascade is handled by xyflow at each level
  // since every draggable child declares `expandParent: true`.
  function onNodeDrag(evt: { targetNode: SFNode | null; event: MouseEvent | TouchEvent }) {
    const node = evt.targetNode;
    if (!node || !node.parentId || !flowApi) return;

    // Start cascade from grandparent — xyflow already handled immediate parent.
    const immediateParent = flowApi.getNode(node.parentId);
    let cursor: string | undefined = immediateParent?.parentId;
    while (cursor) {
      const parent = flowApi.getNode(cursor);
      if (!parent) break;
      const parentW = (parent.width as number) ?? 0;
      const parentH = (parent.height as number) ?? 0;

      let maxX = 0, maxY = 0;
      for (const k of flowApi.getNodes()) {
        if (k.parentId !== cursor) continue;
        const w = (k.width as number) ?? 0;
        const h = (k.height as number) ?? 0;
        const x = k.position?.x ?? 0;
        const y = k.position?.y ?? 0;
        if (x + w > maxX) maxX = x + w;
        if (y + h > maxY) maxY = y + h;
      }

      let newW = parentW, newH = parentH;
      if (maxX + EDGE_GROW_THRESHOLD > parentW) {
        newW = Math.ceil(maxX + EDGE_GROW_THRESHOLD + EDGE_GROW_STEP);
      }
      if (maxY + EDGE_GROW_THRESHOLD > parentH) {
        newH = Math.ceil(maxY + EDGE_GROW_THRESHOLD + EDGE_GROW_STEP);
      }

      if (newW !== parentW || newH !== parentH) {
        flowApi.updateNode(cursor, { width: newW, height: newH });
        cursor = parent.parentId;
      } else {
        break;
      }
    }
  }

  // On release, contract each ancestor back to the tight bbox + padding.
  // This is symmetric with growth: if the tight bbox's min(x,y) drifted
  // past the left/top padding, we shift all siblings back AND shift the
  // parent origin the opposite way, so children keep their screen
  // positions. Then we size width/height to the compacted bbox.
  function onNodeDragStop(evt: { targetNode: SFNode | null; event: MouseEvent | TouchEvent }) {
    const node = evt.targetNode;
    if (!node || !node.parentId || !flowApi) return;

    let cursor: string | undefined = node.parentId;
    while (cursor) {
      const parent = flowApi.getNode(cursor);
      if (!parent) break;
      const siblings = flowApi.getNodes().filter(k => k.parentId === cursor);
      if (siblings.length === 0) break;

      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const c of siblings) {
        const w = (c.width as number) ?? 0;
        const h = (c.height as number) ?? 0;
        const x = c.position?.x ?? 0;
        const y = c.position?.y ?? 0;
        if (x < minX) minX = x;
        if (y < minY) minY = y;
        if (x + w > maxX) maxX = x + w;
        if (y + h > maxY) maxY = y + h;
      }
      if (!isFinite(minX)) break;

      const shiftX = FRAME_PAD.left - minX;
      const shiftY = FRAME_PAD.top - minY;
      if (shiftX !== 0 || shiftY !== 0) {
        for (const c of siblings) {
          flowApi.updateNode(c.id, {
            position: {
              x: (c.position?.x ?? 0) + shiftX,
              y: (c.position?.y ?? 0) + shiftY,
            },
          });
        }
        const pp = parent.position ?? { x: 0, y: 0 };
        flowApi.updateNode(cursor, {
          position: { x: pp.x - shiftX, y: pp.y - shiftY },
        });
      }

      const tightW = maxX - minX;
      const tightH = maxY - minY;
      const newW = Math.max(MIN_FRAME_W, Math.ceil(tightW + FRAME_PAD.left + FRAME_PAD.right));
      const newH = Math.max(MIN_FRAME_H, Math.ceil(tightH + FRAME_PAD.top + FRAME_PAD.bottom));
      const curW = (parent.width as number) ?? 0;
      const curH = (parent.height as number) ?? 0;
      if (curW !== newW || curH !== newH) {
        flowApi.updateNode(cursor, { width: newW, height: newH });
      }
      cursor = parent.parentId;
    }
  }

  // ─── Re-layout signal ────────────────────────────────────────────────────────
  let lastLayoutSignal = 0;
  $effect(() => {
    const v = appState.layoutSignal;
    if (v !== lastLayoutSignal && v > 0) {
      lastLayoutSignal = v;
      untrack(() => {
        buildAndLayout(
          appState.nodes, appState.edges,
          appState.variables, appState.varConsumers,
          appState.groups, appState.groupNodes,
          null,
        );
      });
    }
  });
</script>

<SvelteFlowProvider>
  <FlowApiCapture onready={(api) => flowApi = api} />
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
      onnodedrag={onNodeDrag}
      onnodedragstop={onNodeDragStop}
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
