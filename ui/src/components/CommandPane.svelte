<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges } from "../lib/store";
  let line = "";
  let err = "";

  async function add() {
    err = "";
    try {
      await ipc.addCommand(line.trim());
      line = "";
      const snap = await ipc.snapshot();
      nodes.set(snap.nodes); edges.set(snap.edges);
    } catch (e) { err = String(e); }
  }
</script>

<div class="pane">
  <label class="lbl">New command</label>
  <textarea bind:value={line} rows="3" placeholder="az network vnet create --name v --resource-group rg"></textarea>
  <button on:click={add} disabled={!line.trim()}>Add</button>
  {#if err}<div class="err">{err}</div>{/if}

  <label class="lbl">Commands ({$nodes.filter(n => n.origin === "Declared").length})</label>
  <ul>
    {#each $nodes.filter(n => n.origin === "Declared") as n}
      <li>{n.kind} · {n.name}</li>
    {/each}
  </ul>
</div>

<style>
  .pane { padding:10px; background:#fafafa; height:100%; box-sizing:border-box; overflow:auto; }
  .lbl { display:block; font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin:10px 0 4px; }
  textarea { width:100%; font-family:monospace; font-size:12px; box-sizing:border-box; }
  button { margin-top:6px; width:100%; padding:6px; }
  ul { list-style:none; padding:0; font-family:monospace; font-size:12px; }
  li { padding:2px 0; }
  .err { color:#b53030; font-size:12px; margin-top:6px; white-space:pre-wrap; }
</style>
