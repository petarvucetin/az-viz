<script lang="ts">
  import { appState } from "../lib/store.svelte";
  import { ipc } from "../lib/ipc";
  import { cidrToRange } from "../lib/cidr";

  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  let selected = $derived(appState.nodes.find(n => keyOf(n.id) === appState.selectedNodeKey) ?? null);
  let statusKind = $derived(selected?.status.kind ?? "");
  let isDeclared = $derived(selected?.origin === "Declared");
  let isRunning = $derived(statusKind === "running");
  let isVerifying = $derived(statusKind === "verifying");

  let cidrs = $derived.by(() => {
    const raw = selected?.props?.cidr;
    if (typeof raw === "string") return [raw];
    if (Array.isArray(raw)) return raw.filter((x): x is string => typeof x === "string");
    return [];
  });

  let otherProps = $derived.by(() => {
    const out: Array<[string, string]> = [];
    const p = selected?.props ?? {};
    for (const [k, v] of Object.entries(p)) {
      if (k === "cidr") continue;
      if (typeof v === "string") out.push([k, v]);
      else if (typeof v === "boolean") out.push([k, v ? "yes" : "no"]);
      else if (Array.isArray(v)) out.push([k, v.filter(x => typeof x === "string").join(", ")]);
    }
    return out;
  });

  let showRemove = $derived(!!selected && isDeclared);
  let showVerify = $derived(!!selected);
  let showExecute = $derived(!!selected && isDeclared);
  let removeDisabled = $derived(!selected || isRunning || !selected.command_id);
  let verifyDisabled = $derived(!selected || isRunning || isVerifying);
  let executeDisabled = $derived(!selected || isRunning);

  async function refreshSnapshot() {
    const snap = await ipc.snapshot();
    appState.nodes = snap.nodes;
    appState.edges = snap.edges;
  }

  async function remove() {
    if (!selected?.command_id) return;
    try { await ipc.removeCommand(selected.command_id); await refreshSnapshot(); appState.selectedNodeKey = null; }
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

      {#if otherProps.length > 0}
        <div class="divider"></div>
        {#each otherProps as [k, v]}
          <div class="row"><span class="k">{k}</span><span class="v">{v}</span></div>
        {/each}
      {/if}
    </div>

    <div class="actions">
      {#if showRemove}
        <button class="btn destructive" onclick={remove} disabled={removeDisabled}>Remove</button>
      {/if}
      {#if showVerify}
        <button class="btn" onclick={verify} disabled={verifyDisabled}>Check Azure</button>
      {/if}
      {#if showExecute}
        <button class="btn" onclick={execute} disabled={executeDisabled}>Execute</button>
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
