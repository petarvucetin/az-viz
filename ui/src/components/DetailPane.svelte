<script lang="ts">
  import { nodes, edges, selectedNodeKey, lastError } from "../lib/store";
  import { ipc } from "../lib/ipc";

  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  $: selected = $nodes.find(n => keyOf(n.id) === $selectedNodeKey) ?? null;
  $: statusKind = selected?.status.kind ?? "";
  $: isDeclared = selected?.origin === "Declared";
  $: isRunning = statusKind === "running";
  $: isVerifying = statusKind === "verifying";

  // Availability per spec §5.4 — hidden vs disabled.
  $: showRemove = !!selected && isDeclared;
  $: showVerify = !!selected;
  $: showExecute = !!selected && isDeclared;
  $: removeDisabled = !selected || isRunning || !selected.command_id;
  $: verifyDisabled = !selected || isRunning || isVerifying;
  $: executeDisabled = !selected || isRunning;

  async function refreshSnapshot() {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes);
    edges.set(snap.edges);
  }

  async function remove() {
    if (!selected?.command_id) return;
    try { await ipc.removeCommand(selected.command_id); await refreshSnapshot(); selectedNodeKey.set(null); }
    catch { /* lastError set by wrapper */ }
  }

  async function verify() {
    if (!selected) return;
    try { await ipc.verifyNode(keyOf(selected.id)); await refreshSnapshot(); }
    catch { /* lastError set by wrapper */ }
  }

  async function execute() {
    if (!selected) return;
    try { await ipc.executeNode(keyOf(selected.id)); await refreshSnapshot(); }
    catch { /* lastError set by wrapper */ }
  }
</script>

<div class="pane">
  <div class="lbl">Selected node</div>
  {#if selected}
    <div class="mono">
      <div><b>{selected.kind} · {selected.name}</b></div>
      <div class="muted">rg: {selected.scope.resource_group}</div>
      {#if selected.scope.subscription}<div class="muted">sub: {selected.scope.subscription}</div>{/if}
      {#if selected.scope.location}<div class="muted">loc: {selected.scope.location}</div>{/if}
      <div class="muted">origin: {selected.origin}</div>
      <div class="muted">status: {statusKind}</div>
    </div>
    <div class="actions">
      {#if showRemove}
        <button class="btn destructive" on:click={remove} disabled={removeDisabled}>Remove</button>
      {/if}
      {#if showVerify}
        <button class="btn" on:click={verify} disabled={verifyDisabled}>Check Azure</button>
      {/if}
      {#if showExecute}
        <button class="btn" on:click={execute} disabled={executeDisabled}>Execute</button>
      {/if}
    </div>
  {:else}
    <div class="muted">No node selected</div>
  {/if}
</div>

<style>
  .pane { padding:10px; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .mono { font-family:monospace; font-size:12px; line-height:1.5; }
  .muted { color:#666; }
  .actions { display:flex; flex-direction:column; gap:4px; margin-top:12px; }
  .btn {
    padding:5px 10px; font-size:12px;
    background:#f5f5f5; border:1px solid #ccc; border-radius:3px;
    cursor:pointer; text-align:left;
  }
  .btn:hover:not([disabled]) { background:#eef3fb; border-color:#4a90e2; }
  .btn[disabled] { opacity:.45; cursor:default; }
  .btn.destructive { color:#b53030; }
  .btn.destructive:hover:not([disabled]) { background:#fdf0f0; border-color:#b53030; }
</style>
