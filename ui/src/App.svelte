<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";
  import Toolbar from "./components/Toolbar.svelte";
  import CommandPane from "./components/CommandPane.svelte";
  import GraphCanvas from "./components/GraphCanvas.svelte";
  import DetailPane from "./components/DetailPane.svelte";
  import LogPane from "./components/LogPane.svelte";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<div class="app">
  <Toolbar />
  <div class="grid">
    <CommandPane />
    <div class="canvas-wrap"><GraphCanvas /></div>
    <div class="right">
      <DetailPane />
      <div class="divider" />
      <LogPane />
    </div>
  </div>
</div>

<style>
  :global(html, body, #app) { height:100%; margin:0; }
  .app { display:flex; flex-direction:column; height:100vh; font-family:system-ui, sans-serif; }
  .grid { flex:1; display:grid; grid-template-columns: 280px 1fr 300px; min-height:0; }
  .canvas-wrap { background:#fff; border-left:1px solid #ddd; border-right:1px solid #ddd; min-height:0; }
  .right { display:grid; grid-template-rows: auto 1px 1fr; background:#fafafa; min-height:0; }
  .divider { background:#ddd; }
</style>
