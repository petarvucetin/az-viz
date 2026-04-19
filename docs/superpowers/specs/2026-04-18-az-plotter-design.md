# az-plotter — Design Spec

**Date:** 2026-04-18
**Status:** Design approved; implementation planning to follow.

## 1. Problem

Azure network infrastructure is often set up by running sequences of `az network`
CLI commands. The correct order is dictated by resource dependencies — a subnet
cannot be created before its VNet, an NSG rule cannot be created before its
NSG, and so on. Today, operators maintain this ordering by hand, in shell
scripts that are hard to visualize and easy to get wrong.

`az-plotter` lets an operator enter `az network …` commands one at a time,
shows the resulting resource dependency graph as a DAG as they go, and then
executes the commands in topological order with a single click.

## 2. Goals (v1)

- Parse `az network … create` commands as they are typed or pasted into the UI.
- Build a DAG where each node is an Azure resource and each edge is a
  dependency inferred from a command flag (e.g., `--vnet-name` creates an
  edge from the referenced VNet to the subnet being created).
- Render the DAG live, with distinct visuals for **declared** nodes (have an
  originating command) and **ghost** nodes (pre-existing resources referenced
  but not declared in the current session).
- Verify ghost nodes asynchronously in the background by calling
  `az <kind> show`, and surface the result as a badge on the node.
- Execute the graph in three modes:
  - **Dry-run** — print the materialized command list in topological order.
  - **Live execute** — spawn `az` per node, stream output, update node status.
  - **Emit script** — write an ordered `.sh` or `.ps1` to disk.
- Persist sessions as a single small JSON file (`.azp`).
- Ship as a packaged desktop app (Tauri) for Windows, ~5 MB installer.

## 3. Non-goals (v1)

Deliberately deferred; design hooks are noted in §12.

- `update` and `delete` verbs.
- `--what-if` mode (Azure preflight check for name conflicts etc.).
- Run-time error policies (fail-fast vs continue-on-independent-branch) and
  parallel execution of independent subtrees. For v1, Live execute is strictly
  sequential and stops at the first failure.
- File import (runbook / .sh / .ps1 ingestion).
- Multi-subscription / cross-subscription graphs.
- Bicep or ARM template export.

## 4. Decisions (locked)

| Dimension | Decision |
|---|---|
| Input source | Live entry in the app UI; no file import for v1. |
| Node model | Each Azure resource is a node. Edges are auto-inferred from reference flags (`--vnet-name`, `--nsg-name`, etc.). |
| Verb scope | `create` only. |
| UI platform | Packaged desktop app — **Tauri** (Rust backend, WebView frontend). |
| Execution | Dry-run + Live execute + Emit script. |
| External references | Auto-created **ghost** nodes (dashed border); a **background** verification worker confirms existence — never blocking the UI. |
| Persistence | Single-file `.azp` JSON, autosaved after every mutation. |

## 5. Architecture

Two processes in one Tauri app:

- **Rust core.** Owns all state. Contains: parser, graph store, topological
  planner, runner, verification worker, autosaver. Only this layer talks to
  the `az` binary.
- **WebView frontend.** Svelte + Cytoscape.js. Pure view — holds no state the
  core could not rebuild. Receives events from the core, sends user intents
  back.

They communicate over Tauri IPC: UI → core is a request/response pair
("add command", "run"); core → UI is an event stream ("node-added",
"node-status-changed", "node-log").

Outside the app:

- The `az` CLI binary, spawned as a child process.
- Azure, reached only via `az`.
- `arg-map.json` — bundled, with an optional user override in app-data.
- Project files (`.azp`) on the user's filesystem.

### Boundaries that matter

- **Graph is the single source of truth.** Every feature (parse, verify,
  execute) mutates the graph; the UI is a view over it.
- **Only the Runner and the Verification worker invoke `az`.** The parser
  is offline and deterministic.
- **Arg-map is data, not code.** New `az` subcommands get supported by editing
  JSON, not by recompiling.

## 6. Data model

### Records (Rust structs, serde-serialized)

```rust
enum NodeKind { Vnet, Subnet, Nsg, NsgRule, PublicIp, Nic,
                Lb, RouteTable, ResourceGroup, /* … extensible */ }

enum NodeStatus {
    // declared nodes
    Draft, Ready,
    Running { pid: u32, started_at: Instant },
    Succeeded { duration: Duration },
    Failed { exit_code: i32, stderr_tail: String, duration: Duration },
    Canceled,
    // ghost nodes
    Unverified, Verifying, Exists, Missing,
}

struct Node {
    id: NodeId,          // stable, e.g., "vnet/prod-hub@rg:shared-rg"
    kind: NodeKind,
    name: String,
    scope: Scope,        // { rg, subscription?, location? }
    origin: Origin,      // Declared | Ghost
    status: NodeStatus,
    command_id: Option<CommandId>,
    props: BTreeMap<String, serde_json::Value>,
}

struct Edge {
    from: NodeId,        // parent (prerequisite)
    to:   NodeId,        // child (dependent)
    via:  String,        // the flag that established the ref, e.g. "--vnet-name"
    kind: EdgeKind,      // Ref | Scope
}

struct Command {
    id: CommandId,
    raw: String,
    tokens: Vec<String>,
    parsed_at: DateTime<Utc>,
    produces: NodeId,
    refs: Vec<NodeId>,
    warnings: Vec<Warning>,
}
```

### Node identity (`NodeId`)

A node's identity is `(kind, name, scope.rg, scope.subscription?)`. This is
what the verification worker's CAS (compare-and-swap) staleness check
compares against. Renaming a node, or moving it to a different resource
group, mints a new `NodeId` and invalidates any in-flight verification.

## 7. Parser and arg-map

### Arg-map (excerpt, JSON)

```json
{
  "network vnet create": {
    "produces": { "kind": "vnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  },
  "network vnet subnet create": {
    "produces": { "kind": "subnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "vnet", "via": "--vnet-name", "required": true }
    ]
  },
  "network nsg rule create": {
    "produces": { "kind": "nsg-rule", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "nsg", "via": "--nsg-name", "required": true }
    ]
  }
}
```

The arg-map is loaded at startup from a bundled file, then merged with an
optional user override at `<app-data>/az-plotter/arg-map.override.json`. A
dev-only binary (`cargo run --bin regen-argmap`) refreshes the bundled file
from `az network --help` output; its diff gets code-reviewed.

### Pipeline (per submitted command line)

1. **Tokenize** via the `shell-words` crate. Handles quoting and escapes.
   Reject if the first two tokens are not `az network`.
2. **Match subcommand path** against arg-map keys (longest-prefix match of
   `network <group> [<subgroup>…] <verb>`).
3. **Extract the produced node.** `kind` from the arg-map entry; `name` from
   whichever flag is declared in `name_from`; `scope` from `--resource-group`
   / `--subscription` / `--location`.
4. **Resolve each ref.** Look up an existing node matching the ref's
   `(kind, name, scope)`. If found, draw an edge. If not, create a **ghost**
   node and enqueue a verification job.
5. **Cycle check.** Refuse the edge if it would introduce a cycle. (Creates
   shouldn't cycle in practice; this is a bug-defense.)
6. **Commit** to the graph store; emit `node-added` / `edge-added` events.
7. **Warnings** are attached to the `Command` record: unknown flags,
   references to ghosts, overridden properties on repeat commands.

The parser is pure — given a graph snapshot and a command line, it returns a
deterministic delta. This makes unit testing table-driven and hermetic.

## 8. Runner and execution pipeline

All three run modes share the first four stages; only the fifth differs.

1. **Snapshot.** Freeze the graph at the moment Run is pressed.
2. **Validate.** Block the run with a diagnostic if any declared node has a
   required ref that is unresolved, or that resolves to a ghost whose
   verification status is `Missing`. Ghosts with status `Exists` or
   `Unverified` are accepted (the latter is treated optimistically in v1 —
   the user can wait for verification before pressing Run if they want
   certainty).
3. **Plan.** Kahn's topological sort. Fail with a "cycle detected" diagnostic
   if the planner can't produce a linear order.
4. **Materialize.** For each command, produce the final `az …` argv vector
   (scope flags applied, references resolved to names).
5. **Dispatch** on mode:
   - **Dry-run** — print the materialized list to the log pane, tagged by
     node id. No side effects.
   - **Live execute** — for each command in order: spawn `az` via
     `tokio::process::Command`; stream stdout/stderr into the log pane;
     transition node status Ready → Running → Succeeded/Failed. Sequential
     in v1. Stops at first failure (v1 policy).
   - **Emit script** — write a `.sh` or `.ps1` file (user picks) with a
     header comment (source `.azp`, timestamp, tool version), `set -e` /
     `$ErrorActionPreference = "Stop"`, and one command per line.

### Node lifecycle during live execute

```
Ready
  → Running { pid, started_at }
  → [stdout/stderr lines → node-log events, tagged by node id]
  → on exit(0):  Succeeded { duration }
  → on exit≠0:   Failed { exit_code, stderr_tail, duration }
  → on timeout:  Failed { exit_code: -1, stderr_tail: "timeout", … }
```

### Cancellation

- **Stop** button is active during Live execute. It sends SIGTERM to the
  current child process and marks all remaining pending nodes as `Canceled`.
- Verification jobs carry a `CancellationToken` keyed to the node's current
  identity; renaming / moving a node cancels its in-flight verify.

## 9. Verification worker

- Single Tokio task, bounded MPSC queue.
- Each ghost node creation enqueues a `VerifyJob { node_id, ref_key }`, where
  `ref_key` is a hash of the node's current identity tuple.
- The worker pops a job, spawns
  `az <kind> show --name <name> --resource-group <rg> --output none` with a
  short timeout (default 10 s).
- **Staleness check (CAS).** Before applying the result to the graph, the
  worker re-reads the node's current `ref_key`. If it does not match the
  job's, the result is **discarded**. This prevents a verify answered for an
  old name from flickering onto a node the user has since renamed.
- On match: set `NodeStatus` to `Exists` or `Missing`; emit event; the UI
  re-renders the badge.
- **Rate limit.** At most *N* verification calls per minute (default 30,
  configurable). Protects the control plane when a user pastes a big script.

## 10. UI layout

Single main window, three panes plus a toolbar.

- **Toolbar (top).** Project name, *Dry-run*, *Run*, *Emit script*, *Stop*.
- **Command pane (left, ~280 px).** Always-visible textarea for new
  commands; list of commands with status glyphs below.
- **Graph canvas (center, flex).** Cytoscape.js. Declared nodes have solid
  borders; ghost nodes have dashed borders and a small verification badge.
  Running nodes glow; succeeded nodes are green-tinted; failed nodes are red.
- **Detail + log pane (right, ~300 px).** Selected node's details, its
  originating command, and an *Edit command* / *Remove* row. Below that, a
  log pane that streams runner output.

### Primary interactions

- Paste/type → *Add* → node appears, edges draw immediately.
- Click a node → right pane shows its properties.
- Double-click a node → inline edit its originating command; re-parses on
  save (edges recomputed).
- Right-click a ghost → *Promote to declared* (opens an empty
  `az … create` template with the ghost's identity pre-filled).
- A *Relayout* toolbar button runs a Dagre-style hierarchical layout over
  the current graph.

## 11. Persistence

- **Project file**: `.azp`, JSON, shape
  `{ version, commands[], ui_state }`.
  The graph is re-derived from the commands on load — never stored — keeping
  the file small and diff-friendly.
- **Autosave**: after every successful parse / edit. Autosaves go to the
  currently open file, or to an untitled scratch file in app-data if none
  is open.
- **Recent projects**: up to 10, stored in app-data config.
- **Not persisted**: run results, log output, verification status. These are
  always recomputed on demand, which keeps `.azp` files stable and
  version-control-friendly.

## 12. Deferred work — and where the v1 design hooks it

| Item | Hook already in place |
|---|---|
| Run error policy + parallelism | `Dispatch` is a strategy; a new policy is a new strategy plus a `Blocked` status variant. |
| `--what-if` mode | Fourth dispatch mode; reuses the Materialize stage, queries Azure per command. |
| `update` / `delete` verbs | Arg-map is already keyed by verb; planner gains a reverse-topo pass for deletes. |
| File import (`.sh`, `.ps1`) | An *Import* menu item that calls the existing parser line-by-line. No new core logic. |
| Multi-subscription | `Scope` already has an optional `subscription` field; runner prefixes `az account set` when it changes. |
| Bicep / ARM export | Additional emit target alongside the shell script. |

## 13. Testing strategy

- **Parser (unit).** Golden-file tests on a `(command, expected_node,
  expected_edges, expected_warnings)` table that covers every arg-map entry
  at least once. Property test asserting parser purity — same input graph +
  same command = same delta.
- **Graph store & planner (unit).** Property tests on randomly generated
  DAGs: every topological order respects edges; every synthesized cycle is
  rejected; CAS staleness drops mismatched verification results.
- **Runner (integration).** A tiny fake `az` binary built into the test
  harness reads a scripted response table from an env var. Dry-run asserts
  the ordered command list; Emit snapshot-tests the generated `.sh` /
  `.ps1`; Live simulates success / non-zero exit / timeout via the fake and
  asserts status transitions plus log events.
- **End-to-end (smoke).** One happy-path Tauri test (WebDriver): paste
  three commands → graph appears → press Run (fake az) → all nodes green.

## 14. Success criteria for v1

- Paste a 10-command hub-and-spoke example → DAG renders in under 500 ms.
- Ghost nodes appear immediately; verification badges settle within a few
  seconds on a warm network.
- Dry-run produces a topologically correct command list; Emit writes a
  runnable script that, executed, creates the same resources Live would.
- Live Run executes against a real Azure subscription end-to-end, with node
  colors reflecting reality.
- Distributable is a single Windows `.msi` or portable `.exe` under
  ~15 MB, with no Python or Node runtime required on the user's machine.

## 15. Open questions

None that block implementation. The deferred items in §12 are intentional,
not unresolved.
