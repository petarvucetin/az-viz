<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { ipc } from "../lib/ipc";
  import { keyOf } from "../lib/blocking";
  import { appState } from "../lib/store.svelte";
  import SettingsPopover from "./SettingsPopover.svelte";

  let version = $state("");
  let verifying = $state(false);

  onMount(async () => {
    try { version = await getVersion(); } catch { /* not running under tauri */ }
  });

  function fit() { appState.fitSignal++; }
  function relayout() { appState.layoutSignal++; }

  async function verifyAll() {
    if (verifying) return;
    const nodes = appState.nodes.filter(n => n.origin === "Declared");
    if (nodes.length === 0) return;
    verifying = true;
    appState.appendLog(`[verify-all] checking ${nodes.length} node(s)`);
    try {
      await Promise.all(nodes.map(async (n) => {
        const label = `${n.kind}/${n.name}`;
        try {
          const status = await ipc.verifyNode(keyOf(n.id));
          appState.appendLog(`[verify] ${label}: ${status.kind}`);
        } catch { /* auth-required via banner, other errors via lastError */ }
      }));
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
      appState.appendLog(`[verify-all] done`);
    } finally { verifying = false; }
  }
</script>

<div class="toolbar">
  <span class="title">az-plotter</span>
  <span class="sep">·</span>
  <button onclick={fit}>Fit</button>
  <button onclick={relayout}>Re-layout</button>
  <button onclick={verifyAll} disabled={verifying || appState.nodes.filter(n => n.origin === "Declared").length === 0}>
    {verifying ? "Verifying…" : "Verify"}
  </button>
  <label class="auto">
    <input type="checkbox" bind:checked={appState.autoCreate} />
    Auto Create
  </label>
  <span class="spacer"></span>
  <SettingsPopover />
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
  .auto { display:flex; align-items:center; gap:5px; font-size:12px; cursor:pointer; user-select:none; }
  .auto input { margin:0; cursor:pointer; }
  .spacer { flex:1; }
  .version { opacity:.55; font-size:11px; font-variant-numeric:tabular-nums; }
</style>
