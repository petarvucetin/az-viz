# Remove Command — Design Spec

**Date:** 2026-04-19
**Status:** Design approved; implementation to follow.

## 1. Problem

Once a command is added to the graph there is no way to take it out. A user who makes a typo, changes their plan, or wants to experiment is stuck rebuilding the whole session or hand-editing the persisted `.azp` file.

## 2. Goals (v1)

- Right-click any **declared** node in the graph canvas to open a small context menu.
- The menu has one action, **Remove**, which deletes the command that produced that node.
- Removal also cleans up the node itself, all edges touching it, and any ghost nodes that were referenced only by the removed command.
- If the produced node has **declared dependents** (other commands reference it), the removal is refused with a clear error message.
- Errors surface in the CommandPane's existing `.err` area.

## 3. Non-goals (v1)

- Undo history / redo stack.
- Cascade-remove with a confirmation dialog ("Also remove 3 subnets?").
- Multi-select or bulk remove.
- Keyboard shortcuts (Delete / Backspace) to trigger removal.
- Right-click on ghost nodes (they have no command to remove; menu does not appear).
- Removing the RG compound frame (it is not a real resource).
- Renaming / editing commands in place.

## 4. Decisions (locked)

| Dimension | Decision |
|---|---|
| Trigger | `cxttap` event on Cytoscape nodes with the `kind` data attribute. |
| Menu style | Custom Svelte popup, plain rectangle, matches app aesthetic. Not `cytoscape-cxtmenu`. |
| Menu anchor | Positioned at the mouse page coordinates, clamped to the viewport. |
| Dismiss | Click outside menu, press Escape, or click the menu item. |
| Multi-prefix VNet | Right-click on any `#pN` visual node removes the single underlying VNet command; all prefix visuals disappear together. |
| Ghost nodes | No context menu appears on right-click of a ghost node. |
| Dependency policy | Refuse if the produced node has declared dependents. Surface a clear error. |
| Ghost cleanup | After a command is removed, each node listed in the removed command's `refs` is evaluated: if it is a ghost (origin = `Ghost`) and no remaining command lists it in its own `refs`, the ghost is deleted. |
| Error display | CommandPane's existing `.err` area. Cleared on next successful add or remove. |
| Command lookup | Frontend uses `node.command_id` (set on every declared node) when issuing the IPC. |

## 5. Architecture

### 5.1 Data flow

```
right-click on node (Cytoscape cxttap)
  │
  ▼
GraphCanvas.svelte sets contextMenu store: { logicalKey, commandId, x, y }
  │
  ▼
App.svelte renders <NodeContextMenu> over everything else
  │
  ▼ (user clicks "Remove")
ipc.removeCommand(commandId)
  │
  ▼
Tauri command remove_command(id) on backend:
  1. Find Command by id. Identify its produces_node_id.
  2. Check: any declared edges OUT of produces_node_id? If yes → error.
  3. Remove produces_node (Graph.remove_node): drop node + all edges touching it.
  4. For each id in command.refs: if ghost and not referenced by any remaining command → remove it.
  5. Drop Command from graph.commands and graph.insertion_order.
  │
  ▼
CommandPane receives fresh snapshot, updates list. On error, lastError store populated.
```

### 5.2 Components and responsibilities

**Backend (Rust):**

| File | Role |
|---|---|
| `src-tauri/src/model/graph.rs` | Add `remove_node(&NodeId) -> Result<(), GraphError>` (drops node and all incident edges). Add `remove_command(&str) -> Option<Command>` (drops from `commands` map and `insertion_order`). |
| `src-tauri/src/ipc/commands.rs` | New `#[tauri::command] fn remove_command(id: String, state)` that orchestrates dependency-check + node removal + ghost cleanup + command removal, then autosaves if a project is open. |
| `src-tauri/src/ipc/mod.rs` | No change (re-exports via glob already). |
| `src-tauri/src/main.rs` | Register `remove_command` in `invoke_handler`. |

**Frontend (TypeScript/Svelte):**

| File | Role |
|---|---|
| `ui/src/lib/store.ts` | New stores: `contextMenu: writable<ContextMenuState \| null>` and `lastError: writable<string \| null>`. |
| `ui/src/lib/ipc.ts` | New wrapper: `removeCommand(id: string): Promise<void>`. Sets `lastError` on failure. |
| `ui/src/components/GraphCanvas.svelte` | `cy.on("cxttap", "node[kind]", handler)` — handler opens menu only if node has `commandId` (declared, not ghost). Position from `ev.originalEvent.clientX/Y`. |
| `ui/src/components/NodeContextMenu.svelte` | **NEW.** Popup component: absolute positioned, one button "Remove". Dismisses on outside click / Escape / click-action. Calls `ipc.removeCommand(...)` and refreshes snapshot. |
| `ui/src/components/CommandPane.svelte` | Subscribe to `lastError`; display the error in the existing `.err` div. Clear on next successful add. |
| `ui/src/App.svelte` | Mount `<NodeContextMenu>` at top level so it layers above the graph. |

### 5.3 Types

**Rust (already exists):**
```rust
pub struct Command { pub id: String, ... pub produces: NodeId, pub refs: Vec<NodeId>, ... }
pub struct Node { ... pub command_id: Option<String>, pub origin: Origin, ... }
```

**TypeScript (new):**
```ts
interface ContextMenuState {
  logicalKey: string;     // e.g. "vnet/net-hub@rg:rg1"
  commandId: string;      // Command.id, e.g. "cmd-abc-…"
  x: number;              // page px
  y: number;
}
```

## 6. Error behavior

| Scenario | Backend response | Frontend behavior |
|---|---|---|
| Command id not found | `Err("command not found")` | Displayed in CommandPane `.err`. |
| Produced node has declared dependents | `Err("Can't remove <node-display>: <dependent-display> depends on it. Remove dependents first.")` | Displayed in CommandPane `.err`. Context menu closes regardless. |
| Autosave fails | Silently ignored (same policy as `add_command`). | — |
| Remove succeeds | `Ok(())` | `lastError` cleared; snapshot refreshed; list re-rendered. |

Only **one** dependent is named in the error message (the first one encountered during the outgoing-edge scan) to keep the message short. If there are more, removing the first still leaves the VNet undeletable and the user will see the next one's name on retry.

## 7. Visual details

### Menu markup (approx.)

```svelte
<div class="ctx-menu" style="left:{x}px; top:{y}px;">
  <button class="ctx-item" on:click={remove}>
    Remove
  </button>
</div>
```

- Background: `#ffffff`, border: `1px solid #ddd`, box-shadow: `0 2px 8px rgba(0,0,0,0.15)`, border-radius: 6px.
- Item: padding `6px 14px`, font-size 12, hover `#f0f4fa`, color `#b53030` for destructive cue.
- Viewport clamp: if `x + menuWidth > window.innerWidth`, shift left by menuWidth; if `y + menuHeight > window.innerHeight`, shift up by menuHeight.

### Dismiss

A full-viewport transparent overlay sits behind the menu (`position: fixed; inset: 0; z-index: just-below-menu`). Its `on:click` closes the menu. `on:keydown` at document level handles Escape. Both set `contextMenu` store to `null`.

## 8. Verification

### Backend unit tests (`graph.rs`)

- `remove_node_drops_all_incident_edges`: add VNet + subnet + edge, remove VNet, expect 0 nodes and 0 edges.
- `remove_node_missing_errors`: remove a non-existent node → `NotFound`.

### Backend unit tests (`commands.rs` via a helper on the session state or integration test in `tests/`)

- `remove_command_deletes_produces_and_ghosts`: command references ghost VNet, remove command → ghost also gone.
- `remove_command_refuses_if_dependent_declared`: VNet + subnet commands; remove VNet → error, graph unchanged.
- `remove_command_keeps_ghost_shared_with_other_command`: two subnet commands both reference ghost VNet, remove one → ghost remains (other still refs).

### Frontend smoke (manual via Tauri dev)

1. Add VNet → add subnet → right-click VNet node → menu appears → click Remove → error in CommandPane.
2. Right-click subnet node → Remove → subnet gone, VNet remains.
3. Right-click VNet → Remove → VNet and its ghost deps (if any) gone, graph empty.
4. Right-click a ghost VNet (created by subnet referring to unknown VNet) → no menu appears.
5. Right-click a multi-prefix VNet `#p1` visual node → Remove → all prefix visuals disappear.
6. Open menu → press Escape → menu closes. Open menu → click elsewhere on graph → menu closes.

## 9. Success criteria

- Right-clicking a declared node shows a menu with **Remove** at the cursor location.
- Removing a command with no dependents deletes it cleanly; graph re-renders immediately.
- Removing a command with declared dependents shows a clear error and leaves the graph untouched.
- Ghost nodes are cleaned up when their last referring command is removed.
- Multi-prefix VNet expansion still treats all prefix visuals as one logical entity for removal.
- Existing add / dry-run / live-run flows are unchanged.
