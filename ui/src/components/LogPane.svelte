<script lang="ts">
  import { appState } from "../lib/store.svelte";
</script>

<div class="pane">
  <div class="lbl">Log</div>
  <pre class="log">{appState.logLines.join("\n")}</pre>
  {#if appState.lastRun}
    <div class="summary">
      Done: {appState.lastRun.succeeded} succeeded · {appState.lastRun.failed} failed
    </div>
  {/if}
</div>

<style>
  .pane { padding:10px; display:flex; flex-direction:column; height:100%; box-sizing:border-box; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .log { flex:1; background:#1e1e1e; color:#d0d0d0; padding:8px; border-radius:4px; font-size:11px; line-height:1.4; overflow:auto; margin:0; white-space:pre-wrap; }
  .summary { font-size:12px; margin-top:6px; }
</style>
