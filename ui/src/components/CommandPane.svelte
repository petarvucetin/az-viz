<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges, lastError } from "../lib/store";
  let line = "";
  let localErr = "";

  /** Split textarea content into logical commands.
   *  - Joins bash-style `\` line continuations
   *  - Skips empty lines and `#` comment lines
   *  - Returns each command as a single-line string, preserving order
   */
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
        // Leave unprocessed lines in the textarea so the user can fix and retry.
        const remaining = cmds.slice(processed);
        line = remaining.join("\n");
        const preview = cmd.length > 80 ? cmd.slice(0, 77) + "..." : cmd;
        localErr = `Line ${processed + 1} (${preview}): ${e}`;
        // Still refresh to show whatever did commit.
        const snap = await ipc.snapshot();
        nodes.set(snap.nodes); edges.set(snap.edges);
        return;
      }
    }

    // All commands processed successfully.
    line = "";
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    lastError.set(null);
  }

  // Display whichever error is most recent — localErr from add-command,
  // or the shared lastError from remove/verify/execute wrappers.
  $: err = localErr || $lastError || "";
</script>

<div class="pane">
  <label class="lbl">New command(s)</label>
  <textarea bind:value={line} rows="6" placeholder="az network vnet create --name v --resource-group rg&#10;az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefixes 10.0.0.0/27"></textarea>
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
