<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges } from "../lib/store";
  let running = false;

  async function dry() {
    const plan = await ipc.dryRun();
    console.log("dry-run plan", plan);
  }
  async function run() {
    running = true;
    try { await ipc.runLive(); } finally { running = false; }
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
  }
</script>

<div class="toolbar">
  <span class="title">az-plotter</span>
  <span class="sep">·</span>
  <button on:click={dry} disabled={running}>Dry-run</button>
  <button on:click={run} disabled={running} class="primary">Run</button>
  <button disabled>Emit script</button>
  <button disabled={!running}>Stop</button>
</div>

<style>
  .toolbar { display:flex; align-items:center; gap:8px; padding:8px 12px;
    background:#2d2d2d; color:#eee; font-size:13px; }
  .title { font-weight:600; }
  .sep { opacity:.5; }
  button { background:#555; color:#eee; border:0; padding:4px 10px; border-radius:3px; cursor:pointer; }
  button.primary { background:#2a8f3d; }
  button:disabled { opacity:.5; cursor:default; }
</style>
