<script lang="ts">
  import { appState } from "../lib/store.svelte";
  import { ipc } from "../lib/ipc";
  import { cidrToRange } from "../lib/cidr";

  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  let selectedVarName = $derived(
    appState.selectedNodeKey?.startsWith("var:") ? appState.selectedNodeKey.slice(4) : null
  );
  let selectedVariable = $derived(
    selectedVarName ? appState.variables.find(v => v.name === selectedVarName) ?? null : null
  );
  let varBodyDraft = $state("");
  let refreshingVar = $state(false);

  // Reset the draft textarea whenever the selection changes.
  $effect(() => {
    const v = selectedVariable;
    if (!v) { varBodyDraft = ""; return; }
    if (v.body.mode === "command") varBodyDraft = "$(" + v.body.argv.join(" ") + ")";
    else if (v.body.mode === "literal") varBodyDraft = v.body.value;
    else varBodyDraft = "";
  });

  async function saveVariable() {
    if (!selectedVariable) return;
    try {
      await ipc.setVariableBody(selectedVariable.name, varBodyDraft);
      appState.appendLog(`[var] ${selectedVariable.name}: body saved`);
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
    } catch { /* lastError set by wrapper */ }
  }

  async function executeVariable() {
    if (!selectedVariable) return;
    refreshingVar = true;
    const name = selectedVariable.name;
    const mode = selectedVariable.body.mode;
    appState.appendLog(
      mode === "command" ? `[var] ${name}: executing $(…)` : `[var] ${name}: resolving literal`
    );
    try {
      const val = await ipc.refreshVariable(name);
      appState.appendLog(`[var] ${name} = ${val ?? "∅"}`);
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
    } catch (e) {
      // `not_logged_in` is surfaced via AuthBanner; suppress its duplicate log.
      const msg = String(e);
      if (msg !== "not_logged_in") {
        const lines = msg.split(/\r?\n/);
        appState.appendLog(`[error] ${lines[0]}`);
        for (const rest of lines.slice(1)) {
          if (rest.trim() !== "") appState.appendLog(`        ${rest}`);
        }
      }
    }
    finally { refreshingVar = false; }
  }

  let executeBtnLabel = $derived.by(() => {
    if (refreshingVar) return selectedVariable?.body.mode === "command" ? "Executing…" : "Resolving…";
    if (!selectedVariable) return "Execute";
    if (selectedVariable.body.mode === "command") return "Execute";
    if (selectedVariable.body.mode === "literal") return "Resolve";
    return "Execute";
  });
  let executeBtnTitle = $derived(
    selectedVariable?.body.mode === "command"
      ? "Run the $(…) command and cache its stdout as this variable's value"
      : selectedVariable?.body.mode === "literal"
        ? "Cache the literal value (no az call)"
        : "Set a value first"
  );

  async function removeVariable() {
    if (!selectedVariable) return;
    try {
      await ipc.removeVariable(selectedVariable.name);
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
      appState.selectedNodeKey = null;
    } catch { /* lastError */ }
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

  // Node-local execution error, shown under the buttons until the next run.
  let execError = $state<string | null>(null);

  type ErrLine =
    | { kind: "blank" }
    | { kind: "text"; text: string }
    | { kind: "kv"; key: string; value: string; indent: boolean };

  /// Break an Azure ARM URL into labeled (key,value) lines.
  function armUrlLines(url: string): ErrLine[] {
    const parts = url.split("/").filter(Boolean);
    const idx = (k: string) => parts.findIndex(p => p.toLowerCase() === k.toLowerCase());
    const subI = idx("subscriptions");
    const rgI = idx("resourceGroups");
    const provI = idx("providers");
    if (subI < 0 || parts.length <= subI + 1) {
      return [{ kind: "text", text: url }];
    }
    const out: ErrLine[] = [];
    out.push({ kind: "kv", key: "Sub", value: parts[subI + 1], indent: true });
    if (rgI >= 0 && parts.length > rgI + 1) {
      out.push({ kind: "kv", key: "RG", value: parts[rgI + 1], indent: true });
    }
    if (provI >= 0) {
      const after = parts.slice(provI + 1);
      if (after.length >= 2) {
        const ns = after[0];
        const tailName = after[after.length - 1];
        const middle = after.slice(1, -1);
        out.push({ kind: "kv", key: "providers", value: `${ns}/${middle.join("/")}`, indent: true });
        out.push({ kind: "kv", key: "resource",  value: tailName, indent: true });
      } else if (after.length === 1) {
        out.push({ kind: "kv", key: "providers", value: after[0], indent: true });
      }
    }
    return out;
  }

  const ARM_URL_RE = /\/subscriptions\/[A-Za-z0-9-]+(?:\/[A-Za-z0-9._/-]+)*/;
  const SECTION_RE = /^(ERROR|Code|Message):/i;

  /// Build a structured line list:
  ///  - Blank line before ERROR:/Code:/Message: sections for readability.
  ///  - Every ARM URL becomes its own indented key/value block.
  function formatExecErrorLines(raw: string): ErrLine[] {
    const out: ErrLine[] = [];
    const rawLines = raw.split(/\r?\n/);
    for (let i = 0; i < rawLines.length; i++) {
      const ln = rawLines[i];
      if (SECTION_RE.test(ln.trim()) && out.length > 0) {
        out.push({ kind: "blank" });
      }
      // Look for an ARM URL inside the line; if found, split into pre/url/post.
      const m = ln.match(ARM_URL_RE);
      if (m) {
        const start = ln.indexOf(m[0]);
        const pre = ln.slice(0, start).trimEnd();
        const post = ln.slice(start + m[0].length).trimStart();
        if (pre) out.push({ kind: "text", text: pre });
        out.push(...armUrlLines(m[0]));
        if (post) out.push({ kind: "text", text: post });
      } else if (ln.trim().length === 0) {
        out.push({ kind: "blank" });
      } else {
        out.push({ kind: "text", text: ln });
      }
    }
    return out;
  }

  let execErrorLines = $derived(execError ? formatExecErrorLines(execError) : []);

  // Clear the local error whenever the user switches nodes.
  $effect(() => {
    appState.selectedNodeKey;
    execError = null;
  });

  async function refreshSnapshot() {
    const snap = await ipc.snapshot();
    appState.applySnapshot(snap);
  }

  async function remove() {
    if (!selected?.command_id) return;
    try { await ipc.removeCommand(selected.command_id); await refreshSnapshot(); appState.selectedNodeKey = null; }
    catch { /* lastError set by wrapper */ }
  }

  async function verify() {
    if (!selected) return;
    const label = `${selected.kind}/${selected.name}`;
    try {
      const status = await ipc.verifyNode(keyOf(selected.id));
      appState.appendLog(`[verify] ${label}: ${status.kind}`);
      await refreshSnapshot();
    }
    catch { /* lastError set by wrapper */ }
  }

  async function execute() {
    if (!selected) return;
    // Each execution clears the prior error before running.
    execError = null;
    try {
      await ipc.executeNode(keyOf(selected.id));
      await refreshSnapshot();
      // If the node ended up in a Failed state, surface its stderr tail here.
      const key = keyOf(selected.id);
      const after = appState.nodes.find(n => keyOf(n.id) === key);
      if (after && after.status.kind === "failed") {
        const tail = (after.status as { stderr_tail?: string }).stderr_tail ?? "";
        const code = (after.status as { exit_code?: number }).exit_code ?? -1;
        execError = tail.trim() ? `Exit ${code}\n${tail}` : `Exit ${code}`;
      }
    } catch (e) {
      const msg = String(e);
      if (msg !== "not_logged_in") execError = msg;
    }
  }

  function rangeFor(c: string): string {
    const r = cidrToRange(c);
    return r ? `${r.first} – ${r.last} (${r.count})` : "";
  }
</script>

<div class="pane">
  {#if selectedVariable}
    <div class="lbl">Variable</div>
    <div class="info">
      <div class="title"><b>${selectedVariable.name}</b></div>
      <div class="row"><span class="k">Origin</span><span class="v">{selectedVariable.origin}</span></div>
      <div class="row"><span class="k">Mode</span><span class="v">{selectedVariable.body.mode}</span></div>
      <div class="divider"></div>
      <div class="vbody">
        <label class="lbl" for="vbody">Value or <code>$(az …)</code> command</label>
        <textarea id="vbody" bind:value={varBodyDraft} rows="3" placeholder="$(az network vnet subnet show -g rg --vnet-name v -n s --query id -o tsv)"></textarea>
      </div>
      <div class="divider"></div>
      <label class="lbl" for="vresolved">Resolved</label>
      <textarea
        id="vresolved"
        class="resolved-box"
        class:empty={selectedVariable.resolved === null || selectedVariable.resolved === undefined}
        readonly
        rows="3"
        placeholder="(not resolved)"
        value={selectedVariable.resolved ?? ""}
      ></textarea>
    </div>
    <div class="actions">
      <button class="btn" onclick={saveVariable}>Save</button>
      {#if selectedVariable.body.mode === "command"}
        <button class="btn" onclick={executeVariable} title={executeBtnTitle}
                disabled={refreshingVar}>
          {executeBtnLabel}
        </button>
      {/if}
      <button class="btn destructive" onclick={removeVariable}>Remove</button>
    </div>
  {:else}
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
    {#if execError}
      <div class="exec-error">
        <div class="exec-error-label">Execution error</div>
        <div class="exec-error-body">
          {#each execErrorLines as ln}
            {#if ln.kind === "blank"}
              <div class="err-blank"></div>
            {:else if ln.kind === "text"}
              <div class="err-text-line">{ln.text}</div>
            {:else}
              <div class="err-kv" class:indent={ln.indent}>
                <span class="err-k">{ln.key}:</span>
                <span class="err-v">{ln.value}</span>
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    <div class="muted">No node selected</div>
  {/if}
  {/if}
</div>

<style>
  .pane { padding:10px; font-family:var(--app-ui-font, system-ui, sans-serif); }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .info { font-size:12px; }
  .title { font-size:13px; margin-bottom:8px; color:#0b2447; }
  .row { display:flex; justify-content:space-between; gap:8px; padding:2px 0; line-height:1.4; }
  .k { color:#666; font-size:11px; text-transform:uppercase; letter-spacing:.03em; flex-shrink:0; }
  .v { font-family:var(--app-mono-font, monospace); font-size:11px; color:#222; text-align:right; word-break:break-all; }
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
  .vbody { margin-top:4px; }
  .vbody textarea { width:100%; box-sizing:border-box; font-family:var(--app-mono-font, monospace); font-size:11px; resize:vertical; }
  .resolved-box {
    width:100%; box-sizing:border-box;
    font-family:var(--app-mono-font, monospace); font-size:11px;
    resize:vertical;
    background:#fff; color:#0b2447;
    border:1px solid #ccc; border-radius:3px;
    padding:4px;
    word-break:break-all;
    margin-top:2px;
  }
  .resolved-box.empty { color:#888; font-style:italic; }
  code { font-size:11px; background:#f5f5f5; padding:1px 3px; border-radius:2px; }
  .exec-error {
    margin-top:10px;
    border:1px solid #f5b5b5;
    background:#fff5f5;
    border-radius:4px;
    padding:6px 8px;
  }
  .exec-error-label {
    font-size:10px; font-weight:700; letter-spacing:.04em; text-transform:uppercase;
    color:#b53030; margin-bottom:4px;
  }
  .exec-error-body {
    margin:0;
    font-family: var(--app-mono-font, monospace);
    font-size:11px;
    color:#7f1d1d;
    max-height: 260px;
    overflow: auto;
  }
  .err-blank { height: 6px; }
  .err-text-line { white-space: pre-wrap; word-break: break-word; }
  .err-kv { display:flex; gap:6px; align-items:baseline; padding:1px 0; }
  .err-kv.indent { padding-left: 16px; }
  .err-k { color:#9a3412; flex-shrink:0; }
  .err-v { font-weight: 700; color:#7f1d1d; word-break: break-all; }
</style>
