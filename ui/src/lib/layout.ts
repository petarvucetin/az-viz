import ELK from "elkjs/lib/elk.bundled.js";

const elk = new ELK();

export interface LayoutNode {
  id: string;
  width: number;
  height: number;
  parent?: string;
}
export interface LayoutEdge {
  id: string;
  source: string;
  target: string;
}
export interface LayoutResult {
  positions: Record<string, { x: number; y: number }>;
  sizes: Record<string, { width: number; height: number }>;
}

export async function runLayout(
  nodes: LayoutNode[],
  edges: LayoutEdge[],
): Promise<LayoutResult> {
  // Build nested graph (children inside parents)
  const byParent: Record<string, LayoutNode[]> = {};
  for (const n of nodes) {
    const key = n.parent ?? "__root__";
    (byParent[key] ??= []).push(n);
  }

  function toElk(parentId: string | undefined): any[] {
    const children = byParent[parentId ?? "__root__"] ?? [];
    return children.map(c => {
      const hasChildren = (byParent[c.id]?.length ?? 0) > 0;
      return {
        id: c.id,
        width: c.width,
        height: c.height,
        children: hasChildren ? toElk(c.id) : undefined,
        layoutOptions: {
          "elk.padding": "[top=32,left=18,bottom=18,right=18]",
          ...(hasChildren ? {
            "elk.algorithm": "mrtree",
            "elk.direction": "DOWN",
            "elk.spacing.nodeNode": "80",
            // "elk.layered.spacing.nodeNodeBetweenLayers": "10",
            "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
            "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
          } : {}),
        },
      };
    });
  }

  const graph = {
    id: "root",
    layoutOptions: {
      // "elk.algorithm": "mrtree",
      // "elk.direction": "DOWN",
     
      // "elk.hierarchyHandling": "INCLUDE_CHILDREN",
    },
    children: toElk(undefined),
    edges: edges.map(e => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };

  const result: any = await elk.layout(graph);

  const positions: Record<string, { x: number; y: number }> = {};
  const sizes: Record<string, { width: number; height: number }> = {};

  // ELK returns positions relative to parent. Svelte Flow also wants positions
  // relative to parent for child nodes (those with parentId set). So we use
  // ELK's native relative positions directly — no offset accumulation needed.
  function walk(node: any) {
    if (node.id && node.id !== "root") {
      positions[node.id] = { x: node.x ?? 0, y: node.y ?? 0 };
      sizes[node.id] = { width: node.width ?? 0, height: node.height ?? 0 };
    }
    if (node.children) {
      for (const c of node.children) walk(c);
    }
  }
  walk(result);

  return { positions, sizes };
}
