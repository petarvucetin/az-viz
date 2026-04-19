# Node Context Menu — Design Spec

**Date:** 2026-04-19
**Status:** Design approved; implementation to follow.

## 1. Problem

The canvas currently has no way to act on an individual node. Users can add commands and run the whole graph in topological order, but they can't remove a single command, check whether one specific resource exists in Azure, or execute just one step. This makes iterative session-building (typo fixes, partial deploys, spot-checks) clumsy.

## 2. Goals (v1)

Right-clicking any node in the graph opens a small context menu with up to three actions:

1. **Remove** — delete the command that produced this node, plus orphaned ghost dependencies.
2. **Check Azure** — run `az <kind> show` against the live subscription and update the node's status based on the exit code.
3. **Execute** — run just this node's `az` command and stream its output through the existing LogPane.

Action availability depends on the node's origin and current status (table in §5.4).

## 3. Non-goals (v1)

- Undo / redo history.
- Cascade-remove with a confirmation dialog.
- Multi-select or bulk actions.
- Keyboard shortcuts (Delete, F5, Enter).
- Batch verification ("Verify all unknown nodes"); `verify/worker.rs` stays dead code for now.
- Persisting verification or execution status across sessions — both are in-memory only.
- Deep verification (matching address-prefix etc.); only existence is checked.
- Cancel an in-flight per-node execution mid-stream.
- Pre-execute dependency-existence check (let Azure surface the error).
- Concurrent per-node execute of multiple commands (serialized via session mutex).

## 4. Decisions (locked)

| Dimension | Decision |
|---|---|
| Trigger | `cxttap` event on Cytoscape nodes with the `kind` data attribute. |
| Menu style | Custom Svelte popup; plain rectangle matching app aesthetic. Not `cytoscape-cxtmenu`. |
| Menu anchor | Positioned at mouse page coordinates, clamped to viewport. |
| Dismiss | Click outside menu, press Escape, or click an item. |
| Multi-prefix VNet | Right-click on any `#pN` visual node acts on the single underlying VNet command. |
| Remove — dependency policy | Refuse if the produced node has declared dependents; first dependent named in the error. |
| Remove — ghost cleanup | Each node listed in the removed command's `refs` is evaluated: if it is a ghost and no remaining command lists it in its own `refs`, the ghost is deleted. |
| Check Azure — command | `az <kind> show --name <name> --resource-group <rg> [--subscription <sub>]`; 10 s timeout. |
| Check Azure — result mapping | Exit 0 → `NodeStatus::Exists`; non-zero → `Missing`; timeout / spawn error → IPC Err, status unchanged. |
| Execute — command | `az <tokens...>` reusing the exact tokens stored on the `Command` record. |
| Execute — event stream | Emits `run-event` Tauri events on the same channel as `run_live`; LogPane already listens. |
| Execute — re-run | Allowed after Succeeded/Failed (Azure CLI `create` is idempotent). |
| Execute — concurrency | One per-node execute at a time per session (mutex in SessionState). |
| Command lookup | Frontend passes `logicalKey` and, for Remove, `commandId` (read from `node.command_id`). |
| Error display | CommandPane's existing `.err` area for all three actions. Cleared on next successful action. |

## 5. Architecture

### 5.1 Data flow

```
right-click on node (Cytoscape cxttap)
  │
  ▼
GraphCanvas.svelte sets contextMenu store:
  { logicalKey, commandId?, origin, status, x, y }
  │
  ▼
App.svelte renders <NodeContextMenu> over everything else.
Menu shows only items valid for (origin, status) per §5.4.
  │
  ▼ (user clicks an item)
ipc.removeCommand(commandId) | ipc.verifyNode(logicalKey) | ipc.executeNode(logicalKey)
  │
  ▼
Tauri command on backend:
  Remove  — dep check, remove_node, ghost sweep, drop Command; return ().
  Verify  — spawn `az <kind> show`, set Exists/Missing, return new status.
  Execute — look up Command.tokens, spawn_az, stream events, update status.
  │
  ▼
  Remove / Verify: snapshot refresh triggers UI re-render.
  Execute: run-event stream drives LogPane; final snapshot refresh on Done.
```

### 5.2 Components and responsibilities

**Backend (Rust):**

| File | Role |
|---|---|
| `src-tauri/src/model/graph.rs` | Add `remove_node(&NodeId) -> Result<(), GraphError>` (drops node + all incident edges). Add `remove_command(&str) -> Option<Command>` (drops from `commands` map + `insertion_order`). Add `node_id_from_key(&str) -> Option<NodeId>` helper (inverse of `NodeId::display`). |
| `src-tauri/src/ipc/commands.rs` | New `remove_command`, `verify_node`, `execute_node` tauri commands. |
| `src-tauri/src/ipc/state.rs` | Add `execute_lock: tokio::sync::Mutex<()>` to `Session` for serialized per-node execute. |
| `src-tauri/src/main.rs` | Register three new handlers in `invoke_handler`. |

**Frontend (TypeScript/Svelte):**

| File | Role |
|---|---|
| `ui/src/lib/store.ts` | New stores: `contextMenu: writable<ContextMenuState \| null>`, `lastError: writable<string \| null>`. |
| `ui/src/lib/ipc.ts` | New wrappers: `removeCommand(id)`, `verifyNode(logicalKey)`, `executeNode(logicalKey)`. Each sets `lastError` on failure. |
| `ui/src/components/GraphCanvas.svelte` | `cy.on("cxttap", "node[kind]", handler)`; mouse coords from `ev.originalEvent.clientX/Y`. Read `origin`, `status`, `commandId` from the visual node's data; write to contextMenu store. Add style rules for `node[status = 'missing']` (orange dashed), `node[status = 'exists']` (green solid), `node[status = 'verifying']` (amber). |
| `ui/src/components/NodeContextMenu.svelte` | **NEW.** Popup with up to three items, conditional on `(origin, status)`. Dismiss via outside-click overlay / Escape / action-click. Handles IPC + snapshot refresh. |
| `ui/src/components/CommandPane.svelte` | Subscribe to `lastError`; display in existing `.err` div. |
| `ui/src/App.svelte` | Mount `<NodeContextMenu>` at top level so it layers above graph. |

### 5.3 Types

**TypeScript (new):**
```ts
interface ContextMenuState {
  logicalKey: string;        // e.g. "vnet/net-hub@rg:rg1"
  commandId: string | null;  // null for ghosts
  origin: "Declared" | "Ghost";
  status: string;            // node.status.kind
  x: number;                 // page px
  y: number;
}
```

**Rust:**

- `Node`, `Command`, `NodeStatus`, `NodeId` are unchanged.
- `SessionState` gains `execute_lock: tokio::sync::Mutex<()>`.

### 5.4 Menu availability

| Node state | Remove | Check Azure | Execute |
|---|---|---|---|
| Declared · Draft/Ready | ✓ | ✓ | ✓ |
| Declared · Running | — (disabled) | — (disabled) | — (disabled) |
| Declared · Succeeded/Failed/Canceled | ✓ | ✓ | ✓ (re-run) |
| Declared · Verifying | ✓ | — (disabled) | ✓ |
| Declared · Exists/Missing | ✓ | ✓ (re-check) | ✓ |
| Ghost · Unverified/Missing | — (hidden) | ✓ | — (hidden) |
| Ghost · Exists | — (hidden) | ✓ (re-check) | — (hidden) |
| Ghost · Verifying | — (hidden) | — (disabled) | — (hidden) |

"Hidden" items do not appear; "disabled" items render greyed out and are unclickable. This keeps the menu compact for ghost nodes (one item) while still showing the full action set for declared nodes.

## 6. Visual styles

New Cytoscape style rules in `GraphCanvas.svelte`:

```js
{ selector: "node[status = 'missing']",
  style: { "border-color": "#ff8c1a", "border-style": "dashed" } },
{ selector: "node[status = 'exists']",
  style: { "border-color": "#2a8f3d" } },
{ selector: "node[status = 'verifying']",
  style: { "border-color": "#b58022" } },
```

Terminal verification statuses (`exists`, `missing`) override the default `origin = 'Ghost'` dashed-grey — a verified ghost stops looking like a pending ghost.

### Menu markup

```svelte
<div class="ctx-menu" style="left:{x}px; top:{y}px;">
  {#if showRemove}
    <button class="ctx-item ctx-destructive" on:click={remove}>Remove</button>
  {/if}
  {#if showVerify}
    <button class="ctx-item" disabled={verifyDisabled} on:click={verify}>Check Azure</button>
  {/if}
  {#if showExecute}
    <button class="ctx-item" disabled={executeDisabled} on:click={execute}>Execute</button>
  {/if}
</div>
```

- Background `#ffffff`, border `1px solid #ddd`, shadow `0 2px 8px rgba(0,0,0,0.15)`, radius 6px.
- Item: `padding: 6px 14px`, font-size 12, hover `#f0f4fa`.
- `.ctx-destructive` color `#b53030` to flag Remove.
- `[disabled]` opacity .45, cursor default.
- Viewport clamp: if `x + menuWidth > innerWidth`, shift left by menuWidth; same for bottom.

### Dismiss

Full-viewport transparent overlay behind the menu (`position: fixed; inset: 0`); click closes. Document-level `keydown` handles Escape. Both set `contextMenu` to `null`.

## 7. Error behavior

| Scenario | Backend | Frontend |
|---|---|---|
| Remove: command id not found | `Err("command not found")` | `.err` in CommandPane. |
| Remove: declared dependent exists | `Err("Can't remove <node>: <dep> depends on it. Remove dependents first.")` | `.err`. Menu closes. Graph unchanged. |
| Verify: spawn error (az missing) | `Err("az not found: ...")` | `.err`. Status unchanged. |
| Verify: timeout (10 s) | `Err("verify timed out")` | `.err`. Status unchanged. |
| Verify: exit 0 | `Ok(NodeStatus::Exists)` | Node border turns green. |
| Verify: exit non-zero | `Ok(NodeStatus::Missing)` | Node border turns orange + dashed. |
| Execute: lock held | `Err("another command is already executing")` | `.err`. |
| Execute: spawn error | `Err("...")` | `.err`. |
| Execute: process exits non-zero | Node status → `Failed`; run-event contains stderr tail. | LogPane shows output; node border turns red. |
| Execute: process exits zero | Node status → `Succeeded`. | Node border turns green. |
| Autosave fails | Silently ignored. | — |

Remove error message names only the **first** dependent found (by `BTreeSet` iteration order). Subsequent dependents surface on retry.

## 8. Verification

### Backend unit tests (`src-tauri/src/model/graph.rs`)

- `remove_node_drops_all_incident_edges` — add VNet + subnet + edge; remove VNet; expect 0 nodes, 0 edges.
- `remove_node_missing_errors` — remove non-existent node → `GraphError::NotFound`.

### Backend integration tests (`src-tauri/tests/`)

- `remove_command_deletes_produces_and_ghosts` — single subnet command creates a ghost VNet; removing the subnet command deletes both.
- `remove_command_refuses_if_dependent_declared` — VNet + subnet commands; remove VNet → Err, graph unchanged.
- `remove_command_keeps_ghost_shared_with_other_command` — two subnet commands share a ghost VNet; removing one leaves the ghost intact.

### Frontend tests

No new unit tests — behavior validated by manual smoke.

### Manual smoke (Tauri dev)

1. Add VNet, right-click → menu shows **Remove**, **Check Azure**, **Execute**.
2. Add a subnet → right-click VNet → **Remove** → error in CommandPane. VNet still present.
3. Right-click subnet → **Remove** → subnet gone. Right-click VNet → **Remove** → VNet gone.
4. Subnet referencing a not-yet-declared VNet shows ghost VNet with dashed-grey border. Right-click ghost → menu shows only **Check Azure**.
5. **Check Azure** on a real resource → green solid border.
6. **Check Azure** on a name that doesn't exist → orange dashed border.
7. **Execute** on a vnet-create command (with a fake `az` in PATH) → node flashes amber → green on success / red on failure. LogPane shows output.
8. While one execute is in flight, right-click another node → Execute item is disabled. After the first finishes, it re-enables.
9. Multi-prefix VNet: right-click any `#pN` → menu applies to the single underlying VNet; Remove deletes all prefix visuals.
10. Open menu → Escape → menu closes. Open menu → click elsewhere on graph → menu closes.

## 9. Success criteria

- Right-click a declared node → menu with Remove / Check Azure / Execute, each enabled per §5.4.
- Right-click a ghost node → menu with only Check Azure.
- Remove cleans up produced node, incident edges, and orphaned ghost refs. Fails loudly (in `.err`) if a declared dependent exists.
- Check Azure drives the node's border: green if exists, orange dashed if missing, amber during the request.
- Execute runs one command, streams output to LogPane, terminates with Succeeded/Failed node state.
- Multi-prefix VNet is treated as one logical entity for all three actions.
- Existing add / dry-run / live-run / emit-script flows are unchanged.
