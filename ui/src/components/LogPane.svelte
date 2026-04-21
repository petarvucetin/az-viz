<script lang="ts">
  import { tick } from "svelte";
  import { appState } from "../lib/store.svelte";

  let logEl: HTMLDivElement | null = $state(null);

  // Timestamp prefix is 19 chars, then space.
  const TS_LEN = 19;

  type Parsed = { ts: string; tag: string | null; rest: string };
  function parseLine(line: string): Parsed {
    const ts = line.slice(0, TS_LEN);
    const body = line.slice(TS_LEN).replace(/^\s/, "");
    const m = body.match(/^\[([a-zA-Z0-9_-]+)\]\s?(.*)$/);
    if (m) return { ts, tag: m[1], rest: m[2] };
    return { ts, tag: null, rest: body };
  }

  let parsedLines = $derived(appState.logLines.map(parseLine));

  // Auto-scroll to bottom whenever new lines arrive.
  $effect(() => {
    appState.logLines.length;
    tick().then(() => {
      if (logEl) logEl.scrollTop = logEl.scrollHeight;
    });
  });
</script>

<div class="pane">
  <div class="lbl">Log</div>
  <div class="log" bind:this={logEl}>
    {#each parsedLines as p}
      <div class="line">
        <span class="ts">{p.ts}</span>
        {#if p.tag}
          <span class="tag tag-{p.tag}">[{p.tag}]</span>
        {/if}
        <span class="rest" class:err-text={p.tag === "error"}>{p.rest}</span>
      </div>
    {/each}
  </div>
  {#if appState.lastRun}
    <div class="summary">
      Done: {appState.lastRun.succeeded} succeeded · {appState.lastRun.failed} failed
    </div>
  {/if}
</div>

<style>
  .pane { padding:10px; display:flex; flex-direction:column; height:100%; box-sizing:border-box; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .log {
    flex:1; background:#1e1e1e; color:#d0d0d0;
    padding:8px; border-radius:4px;
    font-family: var(--app-mono-font, ui-monospace, Menlo, Consolas, monospace);
    font-size:11px; line-height:1.45; overflow:auto; margin:0;
  }
  .line { white-space: pre-wrap; word-break: break-word; display:flex; gap:6px; align-items:baseline; }
  .ts { color:#666; flex-shrink:0; }
  .tag {
    flex-shrink:0;
    display:inline-block;
    padding:0 6px;
    border-radius:8px;
    font-weight:700;
    font-size:10px;
    letter-spacing:.02em;
    color:#111;
    background:#9ca3af;
    border:1px solid #6b7280;
  }
  .tag-added    { background:#bbf7d0; border-color:#22c55e; color:#14532d; }
  .tag-checking { background:#e0e7ff; border-color:#6366f1; color:#312e81; }
  .tag-verify   { background:#cffafe; border-color:#06b6d4; color:#164e63; }
  .tag-verify-all { background:#bae6fd; border-color:#0ea5e9; color:#0c4a6e; }
  .tag-cleared  { background:#fde68a; border-color:#eab308; color:#713f12; }
  .tag-duplicate { background:#e5e7eb; border-color:#6b7280; color:#374151; }
  .tag-comment  { background:#dbeafe; border-color:#3b82f6; color:#1e3a8a; }
  .tag-var      { background:#fed7aa; border-color:#fb923c; color:#7c2d12; }
  .tag-error    { background:#fecaca; border-color:#dc2626; color:#7f1d1d; }
  .rest { flex:1; }
  .err-text { color:#fca5a5; }
  .summary { font-size:12px; margin-top:6px; }
</style>
