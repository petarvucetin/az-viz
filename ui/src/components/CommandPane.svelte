<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/api/dialog";
  import { ipc } from "../lib/ipc";
  import { appState } from "../lib/store.svelte";
  import type { Node as GNode } from "../lib/types";
  // `computeBlocked` follows every incoming reference edge (not just the
  // container-kind edge used by `parentKeyOf` for visual nesting). So a
  // private-dns-link whose --virtual-network isn't declared is correctly
  // marked blocked here, even though it's nested under its zone in the tree.
  import { keyOf, parentKeyOf, computeBlocked } from "../lib/blocking";

  let line = $state("");
  let localErr = $state("");

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
    // Keep blank lines out but RETAIN `#` header lines — the backend needs
    // them to partition following commands into groups.
    return merged.filter(l => l.length > 0);
  }

  async function add() {
    localErr = "";
    const cmds = splitLines(line);
    if (cmds.length === 0) return;

    const results = await ipc.addCommandsBatch(cmds);
    const addedCommandIds: string[] = [];
    const duplicates: Array<{ line_index: number; line: string; produces: string }> = [];
    let errResult: { line_index: number; line: string; message: string } | null = null;
    // Walk results in order so [comment] and [duplicate] logs interleave
    // correctly (matches the input's top-to-bottom structure).
    for (const r of results) {
      if (r.kind === "command") addedCommandIds.push(r.id);
      else if (r.kind === "section") appState.appendLog(`[comment] ${r.title}`);
      else if (r.kind === "duplicate") {
        duplicates.push(r);
        appState.appendLog(`[duplicate] ${r.produces} already declared — skipping "${r.line}"`);
      }
      else if (r.kind === "error") { errResult = r; break; }
    }

    if (errResult) {
      // Keep the offending line + everything after in the textarea so the
      // user can fix it; drop successfully-added lines above.
      line = cmds.slice(errResult.line_index).join("\n");
      const preview = errResult.line.length > 80
        ? errResult.line.slice(0, 77) + "..."
        : errResult.line;
      localErr = `Line ${errResult.line_index + 1} (${preview}): ${errResult.message}`;
    } else if (duplicates.length > 0 && addedCommandIds.length === 0) {
      // Entire batch was duplicates — surface in the input-local error so
      // the textarea banner makes it obvious.
      const first = duplicates[0];
      const preview = first.line.length > 80 ? first.line.slice(0, 77) + "..." : first.line;
      localErr = `Skipped: ${first.produces} already declared (and ${duplicates.length - 1} more)`.replace(" and -1 more", "");
      line = "";
    } else {
      line = "";
    }

    const snap = await ipc.snapshot();
    appState.applySnapshot(snap);
    appState.lastError = null;

    // Fire a background existence check per added command. Non-blocking.
    void autoVerifyAddedCommands(addedCommandIds);
  }

  async function autoVerifyAddedCommands(commandIds: string[]): Promise<void> {
    if (commandIds.length === 0) return;
    const nodesByCmd = new Map<string, GNode>();
    for (const n of appState.nodes) {
      if (n.origin === "Declared" && n.command_id) nodesByCmd.set(n.command_id, n);
    }
    // Log the add for every newly-declared node before firing verifies.
    for (const cid of commandIds) {
      const node = nodesByCmd.get(cid);
      if (node) appState.appendLog(`[added] ${node.kind}/${node.name}`);
    }
    await Promise.all(commandIds.map(async (cid) => {
      const node = nodesByCmd.get(cid);
      if (!node) return;
      const label = `${node.kind}/${node.name}`;
      appState.appendLog(`[checking] ${label}`);
      try {
        const status = await ipc.verifyNode(keyOf(node.id));
        appState.appendLog(`[verify] ${label}: ${status.kind}`);
      } catch {
        // not_logged_in is surfaced via AuthBanner; other errors via lastError.
      }
    }));
    const snap = await ipc.snapshot();
    appState.applySnapshot(snap);
  }

  async function clearAll() {
    if (appState.nodes.length === 0) return;
    const ok = await ask("Remove all commands from the graph?", {
      title: "Clear all",
      type: "warning",
    });
    if (!ok) return;
    try {
      await ipc.clearAll();
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
      appState.selectedNodeKey = null;
      appState.appendLog("[cleared] all commands removed");
    } catch { /* lastError set by wrapper */ }
  }

  let err = $derived(localErr || appState.lastError || "");

  let tree = $derived.by(() => {
    const all = appState.nodes;
    const edges = appState.edges;
    const declaredCount = all.filter(n => n.origin === "Declared").length;
    const nodeByKey: Record<string, GNode> = {};
    for (const n of all) nodeByKey[keyOf(n.id)] = n;

    const childrenByParent: Record<string, string[]> = {};
    for (const n of all) {
      const k = keyOf(n.id);
      const pk = parentKeyOf(n, edges);
      const parent = pk ?? `rg:${n.scope.resource_group}`;
      (childrenByParent[parent] ??= []).push(k);
    }

    // Index variables by name and nest each referenced variable under the
    // consumer's logical key. If a variable has multiple consumers, it
    // appears under each.
    const varByName: Record<string, import("../lib/types").Variable> = {};
    for (const v of appState.variables) varByName[v.name] = v;
    const childVars: Record<string, string[]> = {};
    const referencedVarNames = new Set<string>();
    for (const [consumerKey, names] of Object.entries(appState.varConsumers)) {
      for (const name of names) {
        referencedVarNames.add(name);
        (childVars[consumerKey] ??= []).push(`var:${name}`);
      }
    }
    // Orphan variables (declared but no consumer references them yet) surface
    // under a synthetic top-level "Variables" group so they're still editable.
    const orphanVars = appState.variables
      .filter(v => !referencedVarNames.has(v.name))
      .map(v => `var:${v.name}`);

    const rgs = Array.from(new Set(all.map(n => n.scope.resource_group))).sort();
    const blocked = computeBlocked(all, edges);
    return {
      rgs, childrenByParent, nodeByKey, blocked, declaredCount,
      childVars, varByName, orphanVars,
    };
  });

  let expanded = $state<Record<string, boolean>>({});
  function toggle(k: string) {
    if (expanded[k]) delete expanded[k];
    else expanded[k] = true;
  }

  // Default-expand RG rows and any node that has variable children so the
  // variable is visible the moment the consumer command is added.
  $effect(() => {
    const rgs = tree.rgs;
    const consumerKeys = Object.keys(appState.varConsumers);
    untrack(() => {
      for (const rg of rgs) {
        const k = `rg:${rg}`;
        if (expanded[k] === undefined) expanded[k] = true;
      }
      for (const k of consumerKeys) {
        if (expanded[k] === undefined) expanded[k] = true;
      }
    });
  });
</script>

<div class="pane">
  <label class="lbl">New command(s)</label>
  <textarea bind:value={line} rows="6" placeholder="az network vnet create --name v --resource-group rg&#10;az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefixes 10.0.0.0/27"></textarea>
  <div class="btn-row">
    <button onclick={add} disabled={!line.trim()}>Add</button>
    <button class="secondary" onclick={clearAll} disabled={tree.declaredCount === 0} title="Remove all commands">Clear all</button>
  </div>
  {#if err}<div class="err">{err}</div>{/if}

  {#if tree.orphanVars.length > 0}
    <label class="lbl">Variables (unused)</label>
    <ul class="tree">
      {#each tree.orphanVars as vk (vk)}
        {@render varLeaf(vk)}
      {/each}
    </ul>
  {/if}

  <label class="lbl">Resources ({tree.declaredCount})</label>
  <ul class="tree">
    {#each tree.rgs as rg}
      {@const rgKey = `rg:${rg}`}
      <li class="tnode">
        <div
          class="row rg-row"
          onclick={() => toggle(rgKey)}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(rgKey); } }}
          role="button"
          tabindex="0"
        >
          <span class="caret">{expanded[rgKey] ? "▼" : "▶"}</span>
          <span class="rg-name">{rg}</span>
        </div>
        {#if expanded[rgKey]}
          <ul class="children">
            {#each tree.childrenByParent[rgKey] ?? [] as ck (ck)}
              {@render branch(ck)}
            {/each}
          </ul>
        {/if}
      </li>
    {/each}
  </ul>
</div>

{#snippet varLeaf(varKey: string)}
  {@const name = varKey.slice(4)}
  {@const v = tree.varByName[name]}
  {#if v}
    {@const isGhost = v.origin === "Ghost"}
    <li class="tnode">
      <div class="row" class:selected={varKey === appState.selectedNodeKey} class:dim={isGhost}>
        <span class="caret no-kids"></span>
        <span
          class="leaf"
          onclick={() => appState.selectedNodeKey = varKey}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") appState.selectedNodeKey = varKey; }}
          role="button"
          tabindex="0"
          title={isGhost ? "value not set — click to fill in" : undefined}
        >
          <span class="cmd-kind kind-var">var</span>
          <span class="cmd-name">${name}</span>
        </span>
      </div>
    </li>
  {/if}
{/snippet}

{#snippet branch(key: string)}
  {@const n = tree.nodeByKey[key]}
  {@const kids = tree.childrenByParent[key] ?? []}
  {@const vars = tree.childVars[key] ?? []}
  {#if n}
    {@const ghost = n.origin === "Ghost"}
    {@const blocked = tree.blocked.has(key)}
    {@const hasChildren = kids.length > 0 || vars.length > 0}
    <li class="tnode">
      <div class="row" class:selected={key === appState.selectedNodeKey} class:dim={blocked}>
        {#if hasChildren}
          <span
            class="caret"
            onclick={(e) => { e.stopPropagation(); toggle(key); }}
            onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); toggle(key); } }}
            role="button"
            tabindex="0"
          >{expanded[key] ? "▼" : "▶"}</span>
        {:else}
          <span class="caret no-kids"></span>
        {/if}
        <span
          class="leaf"
          onclick={() => appState.selectedNodeKey = key}
          onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") appState.selectedNodeKey = key; }}
          role="button"
          tabindex="0"
          title={blocked ? (ghost ? "implied (not declared) — cannot execute" : "parent not declared — cannot execute") : undefined}
        >
          <span class="cmd-kind" data-k={n.kind}>{n.kind}</span>
          <span class="cmd-name">{n.name}</span>
        </span>
      </div>
      {#if expanded[key] && hasChildren}
        <ul class="children">
          {#each kids as ck (ck)}
            {@render branch(ck)}
          {/each}
          {#each vars as vk (vk)}
            {@render varLeaf(vk)}
          {/each}
        </ul>
      {/if}
    </li>
  {/if}
{/snippet}

<style>
  .pane { padding:10px; background:#fafafa; height:100%; box-sizing:border-box; overflow:auto; }
  .lbl { display:block; font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin:10px 0 4px; }
  textarea { width:100%; font-family:var(--app-mono-font, monospace); font-size:12px; box-sizing:border-box; }
  .btn-row { display:flex; gap:6px; margin-top:6px; }
  .btn-row button { flex:1; padding:6px; margin:0; }
  .btn-row button.secondary { background:#f5f5f5; color:#444; border:1px solid #ccc; }
  .btn-row button.secondary:hover:not([disabled]) { background:#fdf0f0; border-color:#b53030; color:#b53030; }
  button { margin-top:6px; width:100%; padding:6px; }
  .tree, .children { list-style:none; padding:0; margin:0; font-size:12px; }
  .children { padding-left:14px; border-left:1px dotted #d0d7e2; margin-left:5px; }
  .tnode { position:relative; }
  .row {
    display:flex; align-items:center; gap:6px;
    padding:3px 4px; border-radius:3px;
    cursor:pointer;
    border:1px solid transparent;
    user-select:none;
  }
  .row:hover { background:#eef3fb; }
  .row.selected { background:#dbeafe; border-color:#4a90e2; }
  .row.dim { opacity:0.45; font-style:italic; }
  .row.dim:hover { opacity:0.75; }
  .rg-row { font-weight:700; color:#4a90e2; letter-spacing:.02em; }
  .rg-name { font-family:var(--app-ui-font, system-ui, sans-serif); font-size:12px; }
  .caret {
    display:inline-flex; align-items:center; justify-content:center;
    width:12px; height:12px; flex-shrink:0;
    font-size:8px; color:#6b7280;
  }
  .caret.no-kids { visibility:hidden; }
  .leaf { display:flex; align-items:center; gap:6px; flex:1; min-width:0; cursor:pointer; }
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
  .cmd-kind[data-k="private-dns-link"] { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }
  .cmd-kind.kind-var { background:#fff7ed; color:#9a3412; border-color:#fb923c; }
  .cmd-name { font-family:var(--app-mono-font, monospace); color:#0b2447; font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .err { color:#b53030; font-size:12px; margin-top:6px; white-space:pre-wrap; }
</style>
