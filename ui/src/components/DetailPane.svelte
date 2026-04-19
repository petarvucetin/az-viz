<script lang="ts">
  import { nodes, selectedNodeKey } from "../lib/store";
  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }
  $: selected = $nodes.find(n => keyOf(n.id) === $selectedNodeKey) ?? null;
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
      <div class="muted">status: {selected.status.kind}</div>
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
</style>
