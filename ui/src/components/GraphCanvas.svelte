<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import cytoscape from "cytoscape";
  import dagre from "cytoscape-dagre";
  import { nodes, edges, selectedNodeKey } from "../lib/store";
  import type { Node as GNode, Edge as GEdge } from "../lib/types";

  cytoscape.use(dagre);

  let container: HTMLDivElement;
  let cy: cytoscape.Core | null = null;

  function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  function toElements(ns: GNode[], es: GEdge[]) {
    return [
      ...ns.map(n => ({
        data: { id: keyOf(n.id), label: `${n.kind} · ${n.name}`, origin: n.origin, status: n.status.kind },
      })),
      ...es.map((e, i) => ({
        data: { id: `e${i}`, source: keyOf(e.from), target: keyOf(e.to), via: e.via },
      })),
    ];
  }

  onMount(() => {
    cy = cytoscape({
      container,
      elements: toElements($nodes, $edges),
      layout: { name: "dagre", rankDir: "LR" } as any,
      style: [
        { selector: "node", style: {
          "label": "data(label)", "font-size": 11, "text-valign": "center", "text-halign": "center",
          "background-color": "#eaf3ff", "border-color": "#4a90e2", "border-width": 2, "shape": "round-rectangle",
          "padding": "8px", "width": "label", "height": "label",
        } as any },
        { selector: "node[origin = 'Ghost']", style: {
          "background-color": "#f0f0f0", "border-color": "#888", "border-style": "dashed",
        } as any },
        { selector: "node[status = 'running']", style: { "border-color": "#b58022" } },
        { selector: "node[status = 'succeeded']", style: { "background-color": "#e8f7ec", "border-color": "#2a8f3d" } },
        { selector: "node[status = 'failed']", style: { "background-color": "#fde2e2", "border-color": "#b53030" } },
        { selector: "edge", style: {
          "width": 1.5, "line-color": "#999", "target-arrow-color": "#999", "target-arrow-shape": "triangle",
          "curve-style": "bezier",
        } as any },
      ],
    });
    cy.on("tap", "node", ev => selectedNodeKey.set(ev.target.id()));
  });

  $: if (cy) {
    cy.elements().remove();
    cy.add(toElements($nodes, $edges));
    cy.layout({ name: "dagre", rankDir: "LR" } as any).run();
  }

  onDestroy(() => { cy?.destroy(); cy = null; });
</script>

<div bind:this={container} class="canvas"></div>

<style>
  .canvas { width:100%; height:100%; background:#fff; }
</style>
