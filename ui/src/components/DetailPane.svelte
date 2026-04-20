<script lang="ts">
  import { nodes, edges, selectedNodeKey, lastError } from "../lib/store";
  import { ipc } from "../lib/ipc";
  import { cidrToRange } from "../lib/cidr";

  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  $: selected = $nodes.find(n => keyOf(n.id) === $selectedNodeKey) ?? null;
  $: statusKind = selected?.status.kind ?? "";
  $: isDeclared = selected?.origin === "Declared";
  $: isRunning = statusKind === "running";
  $: isVerifying = statusKind === "verifying";

  // Extract CIDR(s) from props.
  $: cidrs = (() => {
    const raw = selected?.props?.cidr;
    if (typeof raw === "string") return [raw];
    if (Array.isArray(raw)) return raw.filter((x): x is string => typeof x === "string");
    return [];
  })();

  // Availability per spec §5.4.
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

  function rangeFor(c: string): string {
    const r = cidrToRange(c);
    return r ? `${r.first} – ${r.last} (${r.count})` : "";
  }
</script>

<div class="pane">
  <div class="lbl">Selected node</div>
  {#if selected}
    <div class="info">
      <div class="title"><b>{selected.kind} · {selected.name}</b></div>
      <div class="row"><span class="k">Resource group</span><span class="v">{selected.scope.resource_group}</span></div>
      {#if selected.scope.subscription}
        <div class="row"><span class="k">Subscription</span><span class="v">{selected.scope.subscription}</span></div>
      {/if}
      {#if selected.scope.location}
        <div class="row"><span class="k">Location</span><span class="v">{selected.scope.location}</span></div>
      {/if}
      <div class="row"><span class="k">Origin</span><span class="v">{selected.origin}</span></div>
      <div class="row"><span class="k">Status</span><span class="v status-{statusKind}">{statusKind}</span></div>

      {#if cidrs.length > 0}
        <div class="divider"></div>
        {#each cidrs as cidr}
          <div class="row"><span class="k">CIDR</span><span class="v cidr">{cidr}</span></div>
          {#if rangeFor(cidr)}
            <div class="row"><span class="k">Range</span><span class="v range">{rangeFor(cidr)}</span></div>
          {/if}
        {/each}
      {/if}
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
  .pane { padding:10px; font-family:system-ui, sans-serif; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .info { font-size:12px; }
  .title { font-size:13px; margin-bottom:8px; color:#0b2447; }
  .row { display:flex; justify-content:space-between; gap:8px; padding:2px 0; line-height:1.4; }
  .k { color:#666; font-size:11px; text-transform:uppercase; letter-spacing:.03em; flex-shrink:0; }
  .v { font-family:monospace; font-size:11px; color:#222; text-align:right; word-break:break-all; }
  .cidr { color:#c9184a; }
  .range { color:#444; }
  .status-running { color:#b58022; }
  .status-succeeded, .status-exists { color:#2a8f3d; }
  .status-failed { color:#b53030; }
  .status-missing { color:#ff8c1a; }
  .divider { height:1px; background:#e0e0e0; margin:8px 0; }
  .muted { color:#666; font-size:12px; }
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
