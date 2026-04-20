<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { ipc } from "../lib/ipc";
  import { appState } from "../lib/store.svelte";

  let running = $state(false);
  let version = $state("");

  onMount(async () => {
    try { version = await getVersion(); } catch { /* not running under tauri */ }
  });

  async function dry() {
    const plan = await ipc.dryRun();
    console.log("dry-run plan", plan);
  }
  async function run() {
    running = true;
    try { await ipc.runLive(); } finally { running = false; }
    const snap = await ipc.snapshot();
    appState.nodes = snap.nodes; appState.edges = snap.edges;
  }
  function fit() { appState.fitSignal++; }
  function relayout() { appState.layoutSignal++; }
</script>

<div class="toolbar">
  <span class="title">az-plotter</span>
  <span class="sep">·</span>
  <button onclick={dry} disabled={running}>Dry-run</button>
  <button onclick={run} disabled={running} class="primary">Run</button>
  <button onclick={fit}>Fit</button>
  <button onclick={relayout}>Re-layout</button>
  <button disabled>Emit script</button>
  <button disabled={!running}>Stop</button>
  <span class="spacer"></span>
  {#if version}<span class="version">v{version}</span>{/if}
</div>

<style>
  .toolbar { display:flex; align-items:center; gap:8px; padding:8px 12px;
    background:#2d2d2d; color:#eee; font-size:13px; }
  .title { font-weight:600; }
  .sep { opacity:.5; }
  button { background:#555; color:#eee; border:0; padding:4px 10px; border-radius:3px; cursor:pointer; }
  button.primary { background:#2a8f3d; }
  button:disabled { opacity:.5; cursor:default; }
  .spacer { flex:1; }
  .version { opacity:.55; font-size:11px; font-variant-numeric:tabular-nums; }
</style>
