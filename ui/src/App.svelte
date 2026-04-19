<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes);
    edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<main>
  <h1>az-plotter</h1>
  <p>nodes: {$nodes.length} · edges: {$edges.length}</p>
</main>
