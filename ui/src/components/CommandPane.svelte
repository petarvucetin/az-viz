<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges, lastError, selectedNodeKey } from "../lib/store";
  let line = "";
  let localErr = "";

  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  function splitLines(input: string): string[] {
    const rawLines = input.split(/\r?\n/);
    const merged: string[] = [];
    let accum = "";
    for (const raw of rawLines) {
      const t = raw.trimEnd();
      if (t.endsWith("\\")) {
        accum += t.slice(0, -1).trim() + " ";
        continue;
      }
      accum += t.trim();
      if (accum) merged.push(accum);
      accum = "";
    }
    if (accum.trim()) merged.push(accum.trim());
    return merged.filter(l => l && !l.startsWith("#"));
  }

  async function add() {
    localErr = "";
    const cmds = splitLines(line);
    if (cmds.length === 0) return;

    let processed = 0;
    for (const cmd of cmds) {
      try {
        await ipc.addCommand(cmd);
        processed++;
      } catch (e) {
        const remaining = cmds.slice(processed);
        line = remaining.join("\n");
        const preview = cmd.length > 80 ? cmd.slice(0, 77) + "..." : cmd;
        localErr = `Line ${processed + 1} (${preview}): ${e}`;
        const snap = await ipc.snapshot();
        nodes.set(snap.nodes); edges.set(snap.edges);
        return;
      }
    }

    line = "";
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    lastError.set(null);
  }

  $: err = localErr || $lastError || "";
</script>

<div class="pane">
  <label class="lbl">New command(s)</label>
  <textarea bind:value={line} rows="6" placeholder="az network vnet create --name v --resource-group rg&#10;az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefixes 10.0.0.0/27"></textarea>
  <button on:click={add} disabled={!line.trim()}>Add</button>
  {#if err}<div class="err">{err}</div>{/if}

  <label class="lbl">Commands ({$nodes.filter(n => n.origin === "Declared").length})</label>
  <ul class="cmd-list">
    {#each $nodes.filter(n => n.origin === "Declared") as n}
      {@const k = keyOf(n.id)}
      <li
        class:selected={k === $selectedNodeKey}
        on:click={() => selectedNodeKey.set(k)}
        on:keydown={(e) => { if (e.key === "Enter" || e.key === " ") selectedNodeKey.set(k); }}
        role="button"
        tabindex="0"
      >
        <span class="cmd-kind" data-k={n.kind}>{n.kind}</span>
        <span class="cmd-name">{n.name}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .pane { padding:10px; background:#fafafa; height:100%; box-sizing:border-box; overflow:auto; }
  .lbl { display:block; font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin:10px 0 4px; }
  textarea { width:100%; font-family:monospace; font-size:12px; box-sizing:border-box; }
  button { margin-top:6px; width:100%; padding:6px; }
  .cmd-list { list-style:none; padding:0; margin:0; font-size:12px; }
  .cmd-list li {
    display:flex; align-items:center; gap:6px;
    padding:4px 6px; border-radius:3px;
    cursor:pointer;
    border:1px solid transparent;
  }
  .cmd-list li:hover { background:#eef3fb; }
  .cmd-list li.selected { background:#dbeafe; border-color:#4a90e2; }
  .cmd-kind {
    display:inline-block;
    font-size:9px; font-weight:700;
    padding:1px 6px;
    border-radius:8px;
    background:#f3f4f6; color:#374151;
    border:1px solid #9ca3af;
    text-transform:lowercase;
    font-variant-numeric:tabular-nums;
    flex-shrink:0;
  }
  .cmd-kind[data-k="vnet"]          { background:#e0f2fe; color:#0369a1; border-color:#0ea5e9; }
  .cmd-kind[data-k="subnet"]        { background:#dcfce7; color:#15803d; border-color:#22c55e; }
  .cmd-kind[data-k="nsg"]           { background:#fef3c7; color:#92400e; border-color:#f59e0b; }
  .cmd-kind[data-k="nsg-rule"]      { background:#ffedd5; color:#9a3412; border-color:#f97316; }
  .cmd-kind[data-k="public-ip"]     { background:#cffafe; color:#0e7490; border-color:#06b6d4; }
  .cmd-kind[data-k="nic"]           { background:#f3e8ff; color:#6b21a8; border-color:#a855f7; }
  .cmd-kind[data-k="lb"]            { background:#fce7f3; color:#9d174d; border-color:#ec4899; }
  .cmd-kind[data-k="route-table"]   { background:#fef9c3; color:#854d0e; border-color:#eab308; }
  .cmd-kind[data-k="vnet-gateway"]  { background:#e0e7ff; color:#3730a3; border-color:#6366f1; }
  .cmd-kind[data-k="local-gateway"] { background:#ccfbf1; color:#115e59; border-color:#14b8a6; }
  .cmd-kind[data-k="vpn-connection"]{ background:#ffe4e6; color:#9f1239; border-color:#f43f5e; }
  .cmd-kind[data-k="vnet-peering"]  { background:#ecfccb; color:#3f6212; border-color:#84cc16; }
  .cmd-kind[data-k="dns-resolver"]  { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }
  .cmd-kind[data-k="private-dns-zone"] { background:#f5f3ff; color:#4c1d95; border-color:#7c3aed; }
  .cmd-name { font-family:monospace; color:#0b2447; font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .err { color:#b53030; font-size:12px; margin-top:6px; white-space:pre-wrap; }
</style>
