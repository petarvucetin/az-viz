<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { contextMenu, nodes, edges } from "../lib/store";
  import { ipc } from "../lib/ipc";
  import type { ContextMenuState } from "../lib/store";

  let menuEl: HTMLDivElement;
  let x = 0, y = 0;

  $: state = $contextMenu;

  async function refreshSnapshot() {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes);
    edges.set(snap.edges);
  }

  function close() { contextMenu.set(null); }

  async function remove() {
    if (!state?.commandId) return;
    try { await ipc.removeCommand(state.commandId); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  async function verify() {
    if (!state) return;
    try { await ipc.verifyNode(state.logicalKey); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  async function execute() {
    if (!state) return;
    try { await ipc.executeNode(state.logicalKey); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  function isRunning(s: string): boolean { return s === "running"; }
  function isVerifying(s: string): boolean { return s === "verifying"; }

  $: isDeclared = state?.origin === "Declared";
  $: showRemove = isDeclared && state !== null;
  $: showVerify = state !== null;
  $: showExecute = isDeclared && state !== null;
  $: removeDisabled = !!state && (isRunning(state.status) || !state.commandId);
  $: verifyDisabled = !!state && (isRunning(state.status) || isVerifying(state.status));
  $: executeDisabled = !!state && isRunning(state.status);

  // Viewport clamp (rough: 180×140 budget).
  $: if (state) {
    x = Math.min(state.x, window.innerWidth  - 180);
    y = Math.min(state.y, window.innerHeight - 140);
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape" && state) close();
  }

  onMount(() => { window.addEventListener("keydown", onKey); });
  onDestroy(() => { window.removeEventListener("keydown", onKey); });
</script>

{#if state}
  <div class="ctx-overlay" on:click={close} role="presentation"></div>
  <div class="ctx-menu" style="left:{x}px; top:{y}px;" bind:this={menuEl}>
    {#if showRemove}
      <button class="ctx-item ctx-destructive" on:click={remove} disabled={removeDisabled}>Remove</button>
    {/if}
    {#if showVerify}
      <button class="ctx-item" on:click={verify} disabled={verifyDisabled}>Check Azure</button>
    {/if}
    {#if showExecute}
      <button class="ctx-item" on:click={execute} disabled={executeDisabled}>Execute</button>
    {/if}
  </div>
{/if}

<style>
  .ctx-overlay {
    position: fixed; inset: 0; z-index: 999;
    background: transparent;
  }
  .ctx-menu {
    position: fixed; z-index: 1000;
    background: #ffffff;
    border: 1px solid #ddd; border-radius: 6px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
    min-width: 140px;
    padding: 4px 0;
    font-family: system-ui, sans-serif;
  }
  .ctx-item {
    display: block; width: 100%;
    padding: 6px 14px; margin: 0;
    background: transparent; border: 0;
    text-align: left; font-size: 12px; color: #222;
    cursor: pointer;
  }
  .ctx-item:hover:not([disabled]) { background: #f0f4fa; }
  .ctx-destructive { color: #b53030; }
  .ctx-item[disabled] { opacity: 0.45; cursor: default; }
</style>
