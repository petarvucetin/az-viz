# Node Context Menu Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a right-click context menu on graph nodes offering three actions — Remove the command, Check Azure for existence, Execute the single command — with the dependency/cascade/cleanup semantics detailed in the spec.

**Architecture:** Three new Tauri IPC commands (`remove_command`, `verify_node`, `execute_node`) hang off existing infrastructure: `Graph` gains `remove_node` / `remove_command` / `node_id_from_key` helpers; `Session` gains an `execute_lock` mutex; existing `runner::az::spawn_az` is reused for execute. Frontend listens to Cytoscape's `cxttap`, writes a `contextMenu` store, mounts a new `NodeContextMenu.svelte` popup at the app root; errors flow through a shared `lastError` store that CommandPane already has a slot for.

**Tech Stack:** Rust (tokio, serde, tauri v1), Svelte 4, Cytoscape 3, cytoscape-node-html-label, vitest.

---

## Phase 1: Backend — graph helpers

### Task 1: Graph removal helpers + key parser

**Files:**
- Modify: `src-tauri/src/model/graph.rs`
- Modify: `src-tauri/src/model/ids.rs` (add `NodeId::from_key` parser)

- [ ] **Step 1: Write failing tests**

Append to `mod tests` in `src-tauri/src/model/graph.rs`:

```rust
    #[test]
    fn remove_node_drops_all_incident_edges() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let s = Node::for_test(NodeKind::Subnet, "s", "rg");
        let vid = v.id.clone();
        let sid = s.id.clone();
        g.add_node(v).unwrap();
        g.add_node(s).unwrap();
        g.add_edge(Edge { from: vid.clone(), to: sid.clone(), via: "--vnet-name".into(), kind: EdgeKind::Ref }).unwrap();
        assert_eq!(g.nodes().count(), 2);
        assert_eq!(g.edges().count(), 1);
        g.remove_node(&vid).unwrap();
        assert_eq!(g.nodes().count(), 1);
        assert_eq!(g.edges().count(), 0);
        assert!(g.node(&sid).is_some());
    }

    #[test]
    fn remove_node_missing_errors() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let vid = v.id.clone();
        let err = g.remove_node(&vid).unwrap_err();
        assert!(matches!(err, GraphError::NotFound(_)));
    }

    #[test]
    fn remove_command_drops_command_and_preserves_insertion_order() {
        let mut g = mk_graph();
        let c1 = Command {
            id: "cmd-1".into(), raw: "x".into(), tokens: vec![], parsed_at: chrono::Utc::now(),
            produces: NodeId::of(NodeKind::Vnet, "v", &Scope::new("rg")),
            refs: vec![], warnings: vec![],
        };
        let c2 = Command { id: "cmd-2".into(), ..c1.clone() };
        g.add_command(c1);
        g.add_command(c2);
        let removed = g.remove_command("cmd-1");
        assert!(removed.is_some());
        let remaining: Vec<_> = g.commands().map(|c| c.id.clone()).collect();
        assert_eq!(remaining, vec!["cmd-2"]);
        assert!(g.remove_command("cmd-nonexistent").is_none());
    }
```

Append to `mod tests` in `src-tauri/src/model/ids.rs`:

```rust
    #[test]
    fn node_id_from_key_parses_without_subscription() {
        let id = NodeId::from_key("vnet/net-hub@rg:lakeflow").unwrap();
        assert_eq!(id.kind, NodeKind::Vnet);
        assert_eq!(id.name, "net-hub");
        assert_eq!(id.resource_group, "lakeflow");
        assert_eq!(id.subscription, None);
    }

    #[test]
    fn node_id_from_key_parses_with_subscription() {
        let id = NodeId::from_key("subnet/app@rg:rg1/sub:sub-abc").unwrap();
        assert_eq!(id.kind, NodeKind::Subnet);
        assert_eq!(id.name, "app");
        assert_eq!(id.resource_group, "rg1");
        assert_eq!(id.subscription, Some("sub-abc".into()));
    }

    #[test]
    fn node_id_from_key_rejects_bad_input() {
        assert!(NodeId::from_key("").is_none());
        assert!(NodeId::from_key("vnet/v").is_none());             // no @rg
        assert!(NodeId::from_key("bogus/v@rg:r").is_none());       // unknown kind
        assert!(NodeId::from_key("vnet/@rg:r").is_none());         // empty name
    }

    #[test]
    fn node_id_roundtrips_through_display_and_from_key() {
        let scope = Scope { resource_group: "my-rg".into(), subscription: Some("s1".into()), location: None };
        let id = NodeId::of(NodeKind::NsgRule, "rule-a", &scope);
        let back = NodeId::from_key(&id.display()).unwrap();
        assert_eq!(id, back);
    }
```

- [ ] **Step 2: Run tests to verify FAIL**

```bash
cd src-tauri && cargo test --lib model:: 2>&1 | tail -15
```

Expected: compile errors because `remove_node`, `remove_command`, and `NodeId::from_key` don't exist.

- [ ] **Step 3: Implement `Graph::remove_node` and `Graph::remove_command`**

Add these methods to `impl Graph` in `src-tauri/src/model/graph.rs` (after the existing `add_command` method near line 80):

```rust
    pub fn remove_node(&mut self, id: &NodeId) -> Result<Node, GraphError> {
        let node = self.nodes.remove(id).ok_or_else(|| GraphError::NotFound(id.display()))?;
        self.edges.retain(|e| &e.from != id && &e.to != id);
        Ok(node)
    }

    pub fn remove_command(&mut self, id: &str) -> Option<Command> {
        let cmd = self.commands.remove(id)?;
        self.insertion_order.retain(|x| x != id);
        Some(cmd)
    }
```

- [ ] **Step 4: Implement `NodeId::from_key`**

Add a `from_key` associated function to `impl NodeId` in `src-tauri/src/model/ids.rs`:

```rust
    pub fn from_key(s: &str) -> Option<Self> {
        // Inverse of display(): "<kind>/<name>@rg:<rg>[/sub:<sub>]"
        let (kind_name, scope_part) = s.split_once('@')?;
        let (kind_str, name) = kind_name.split_once('/')?;
        if name.is_empty() { return None; }
        let kind = match kind_str {
            "vnet" => NodeKind::Vnet,
            "subnet" => NodeKind::Subnet,
            "nsg" => NodeKind::Nsg,
            "nsg-rule" => NodeKind::NsgRule,
            "public-ip" => NodeKind::PublicIp,
            "nic" => NodeKind::Nic,
            "lb" => NodeKind::Lb,
            "route-table" => NodeKind::RouteTable,
            "rg" => NodeKind::ResourceGroup,
            _ => return None,
        };
        let (rg_part, sub) = match scope_part.split_once("/sub:") {
            Some((rg, sub)) => (rg, Some(sub.to_string())),
            None => (scope_part, None),
        };
        let rg = rg_part.strip_prefix("rg:")?.to_string();
        Some(Self { kind, name: name.to_string(), resource_group: rg, subscription: sub })
    }
```

- [ ] **Step 5: Run tests to verify PASS**

```bash
cd src-tauri && cargo test --lib model:: 2>&1 | tail -15
```

Expected: all model tests pass (pre-existing + 4 new).

- [ ] **Step 6: Run full test suite to confirm no regressions**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/model/graph.rs src-tauri/src/model/ids.rs
git commit -m "model: add remove_node, remove_command, NodeId::from_key helpers"
```

---

## Phase 2: Backend — IPC commands

### Task 2: `remove_command` Tauri command + integration tests

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs` (register handler)
- Create: `src-tauri/tests/context_menu.rs` (integration tests)

- [ ] **Step 1: Write failing integration tests**

Create `src-tauri/tests/context_menu.rs`:

```rust
use az_plotter::ipc::state::Session;
use az_plotter::parser::{ArgMap, commit as commit_parse, parse};
use std::sync::Arc;

fn session() -> Arc<Session> {
    let json = std::fs::read_to_string("arg-map.json").unwrap();
    Arc::new(Session::new(ArgMap::from_json(&json).unwrap()))
}

fn add(s: &Session, line: &str) -> String {
    let mut g = s.graph.lock().unwrap();
    let parsed = parse(line, &s.argmap, &g).unwrap();
    let id = parsed.command.id.clone();
    commit_parse(&mut g, parsed).unwrap();
    id
}

#[test]
fn remove_command_deletes_produces_and_ghosts() {
    let s = session();
    // Subnet referencing a ghost VNet "ghosty" (no vnet create before).
    let cid = add(&s, "az network vnet subnet create --name a --resource-group rg --vnet-name ghosty");
    az_plotter::ipc::commands::do_remove_command(&cid, &s).unwrap();
    let g = s.graph.lock().unwrap();
    assert_eq!(g.nodes().count(), 0, "ghost vnet should be cleaned up with subnet removal");
    assert_eq!(g.commands().count(), 0);
}

#[test]
fn remove_command_refuses_if_declared_dependent_exists() {
    let s = session();
    let vnet_cid = add(&s, "az network vnet create --name v --resource-group rg");
    let _ = add(&s, "az network vnet subnet create --name a --resource-group rg --vnet-name v");
    let err = az_plotter::ipc::commands::do_remove_command(&vnet_cid, &s).unwrap_err();
    assert!(err.contains("depends on"), "error should name the dependent: got {err}");
    let g = s.graph.lock().unwrap();
    assert_eq!(g.nodes().count(), 2, "graph unchanged on dep-refusal");
    assert_eq!(g.commands().count(), 2);
}

#[test]
fn remove_command_keeps_ghost_shared_with_other_command() {
    let s = session();
    let cid_a = add(&s, "az network vnet subnet create --name a --resource-group rg --vnet-name ghosty");
    let _cid_b = add(&s, "az network vnet subnet create --name b --resource-group rg --vnet-name ghosty");
    az_plotter::ipc::commands::do_remove_command(&cid_a, &s).unwrap();
    let g = s.graph.lock().unwrap();
    let kinds: Vec<_> = g.nodes().map(|n| n.kind).collect();
    assert!(kinds.iter().any(|k| matches!(k, az_plotter::model::NodeKind::Vnet)),
        "shared ghost VNet should remain");
    assert!(kinds.iter().any(|k| matches!(k, az_plotter::model::NodeKind::Subnet)),
        "subnet b should remain");
    assert_eq!(g.nodes().count(), 2);
}

#[test]
fn remove_command_unknown_id_errors() {
    let s = session();
    let err = az_plotter::ipc::commands::do_remove_command("cmd-nonexistent", &s).unwrap_err();
    assert!(err.contains("not found"));
}
```

- [ ] **Step 2: Run tests to verify FAIL**

```bash
cd src-tauri && cargo test --test context_menu 2>&1 | tail -15
```

Expected: compile error — `do_remove_command` doesn't exist yet. (Also the `ipc::commands` module may need a `pub` export.)

- [ ] **Step 3: Implement `do_remove_command` + Tauri wrapper**

Append to `src-tauri/src/ipc/commands.rs` (after the existing `run_live` command, before the `RunEventWire` enum):

```rust
/// Pure helper: the removal logic. Returns Err(message) on dep-refusal or not-found.
/// Kept separate from the tauri::command wrapper so integration tests can call it
/// without constructing a full Tauri runtime.
pub fn do_remove_command(id: &str, session: &crate::ipc::state::Session) -> Result<(), String> {
    let mut g = session.graph.lock().map_err(|e| e.to_string())?;
    // Locate the command
    let cmd = g.commands().find(|c| c.id == id).cloned()
        .ok_or_else(|| format!("command not found: {id}"))?;
    let produces = cmd.produces.clone();

    // Dependency check: any DECLARED node has an edge FROM produces?
    // Equivalently: any edge e where e.from == produces AND e.to is a declared node.
    let dependents: Vec<_> = g.children(&produces)
        .filter(|to_id| g.node(to_id).map(|n| matches!(n.origin, crate::model::Origin::Declared)).unwrap_or(false))
        .cloned()
        .collect();
    if let Some(dep) = dependents.first() {
        return Err(format!(
            "Can't remove {}: {} depends on it. Remove dependents first.",
            produces.display(), dep.display()
        ));
    }

    // Remove the produced node (drops all incident edges).
    g.remove_node(&produces).map_err(|e| e.to_string())?;

    // Remove the command record.
    g.remove_command(id);

    // Ghost cleanup: any node listed in the removed command's refs that is
    // (a) a ghost AND (b) no remaining command lists it in its refs → remove.
    for ref_id in cmd.refs.iter() {
        let is_ghost = g.node(ref_id).map(|n| matches!(n.origin, crate::model::Origin::Ghost)).unwrap_or(false);
        if !is_ghost { continue; }
        let still_referenced = g.commands().any(|c| c.refs.contains(ref_id));
        if !still_referenced {
            let _ = g.remove_node(ref_id);
        }
    }

    // Autosave if a project is open (same fire-and-forget policy as add_command).
    drop(g);
    if let Some(path) = session.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let g = session.graph.lock().map_err(|e| e.to_string())?;
        let _ = crate::persist::ProjectFile::from_graph(&g).save(path);
    }
    Ok(())
}

#[tauri::command]
pub fn remove_command(id: String, state: tauri::State<SessionState>) -> Result<(), String> {
    do_remove_command(&id, state.inner())
}
```

Ensure the `pub fn do_remove_command` is visible from the integration test. The test uses `az_plotter::ipc::commands::do_remove_command`. Confirm `src-tauri/src/ipc/mod.rs` re-exports appropriately (should already `pub use commands`).

- [ ] **Step 4: Register `remove_command` in main.rs**

In `src-tauri/src/main.rs`, extend the `invoke_handler` list:

```rust
        .invoke_handler(tauri::generate_handler![
            ipc_cmd::add_command,
            ipc_cmd::snapshot,
            ipc_cmd::dry_run,
            ipc_cmd::emit_script,
            ipc_cmd::open_project,
            ipc_cmd::save_project_as,
            ipc_cmd::run_live,
            ipc_cmd::remove_command,
        ])
```

- [ ] **Step 5: Run integration tests to verify PASS**

```bash
cd src-tauri && cargo test --test context_menu 2>&1 | tail -10
```

Expected: all 4 tests pass.

- [ ] **Step 6: Run full test suite**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ipc/commands.rs src-tauri/src/main.rs src-tauri/tests/context_menu.rs
git commit -m "ipc: remove_command — dep check, ghost cleanup, autosave"
```

---

### Task 3: `verify_node` Tauri command

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement `do_verify_node` + Tauri wrapper**

Append to `src-tauri/src/ipc/commands.rs` (after `remove_command`):

```rust
/// Run `az <kind> show --name N --resource-group RG [--subscription S]` and update
/// `node.status` to `Exists` or `Missing` based on exit code. 10 s timeout.
pub async fn do_verify_node(
    logical_key: &str,
    session: &crate::ipc::state::Session,
) -> Result<crate::model::NodeStatus, String> {
    use crate::model::{NodeId, NodeKind, NodeStatus};
    use crate::runner::{spawn_az, AzConfig, AzEvent};
    use tokio::sync::{mpsc, oneshot};

    let node_id = NodeId::from_key(logical_key)
        .ok_or_else(|| format!("bad logical key: {logical_key}"))?;

    // Mark as Verifying and drop the lock before spawning.
    {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        match g.node_mut(&node_id) {
            Some(n) => n.status = NodeStatus::Verifying,
            None => return Err(format!("node not found: {logical_key}")),
        }
    }

    // Build the az argv.
    let kind_str = match node_id.kind {
        NodeKind::Vnet => "vnet",
        NodeKind::Subnet => "vnet subnet",
        NodeKind::Nsg => "nsg",
        NodeKind::NsgRule => "nsg rule",
        NodeKind::PublicIp => "public-ip",
        NodeKind::Nic => "nic",
        NodeKind::Lb => "lb",
        NodeKind::RouteTable => "route-table",
        NodeKind::ResourceGroup => {
            // `az group show` (not `az network rg show`).
            let mut argv = vec!["group".to_string(), "show".into(), "--name".into(), node_id.name.clone()];
            if let Some(ref sub) = node_id.subscription { argv.extend(["--subscription".into(), sub.clone()]); }
            return run_and_classify(&node_id, argv, session).await;
        }
    };
    let mut argv: Vec<String> = "network".split_whitespace().map(String::from).collect();
    for part in kind_str.split_whitespace() { argv.push(part.to_string()); }
    argv.push("show".into());
    argv.extend(["--name".into(), node_id.name.clone(),
                 "--resource-group".into(), node_id.resource_group.clone()]);
    if let Some(ref sub) = node_id.subscription { argv.extend(["--subscription".into(), sub.clone()]); }

    run_and_classify(&node_id, argv, session).await
}

async fn run_and_classify(
    node_id: &crate::model::NodeId,
    argv: Vec<String>,
    session: &crate::ipc::state::Session,
) -> Result<crate::model::NodeStatus, String> {
    use crate::model::NodeStatus;
    use crate::runner::{spawn_az, AzConfig, AzEvent};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    let cfg = AzConfig { exe: "az".into(), timeout: Duration::from_secs(10) };
    let (tx, mut rx) = mpsc::channel::<AzEvent>(32);
    let (_cancel_tx, cancel_rx) = oneshot::channel();
    spawn_az(&cfg, &argv, tx, cancel_rx).await;

    // Drain the channel looking for an Exit event.
    let mut exit_code: Option<i32> = None;
    let mut timed_out = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AzEvent::Exit { code, .. } => { exit_code = Some(code); break; }
            AzEvent::Timeout => { timed_out = true; break; }
            _ => {}
        }
    }

    let new_status = if timed_out {
        // Roll status back to Unverified and error out.
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(node_id) { n.status = NodeStatus::Unverified; }
        return Err("verify timed out".into());
    } else if exit_code == Some(0) {
        NodeStatus::Exists
    } else {
        NodeStatus::Missing
    };

    let mut g = session.graph.lock().map_err(|e| e.to_string())?;
    if let Some(n) = g.node_mut(node_id) { n.status = new_status.clone(); }
    Ok(new_status)
}

#[tauri::command]
pub async fn verify_node(
    logical_key: String,
    state: tauri::State<'_, SessionState>,
) -> Result<crate::model::NodeStatus, String> {
    do_verify_node(&logical_key, state.inner()).await
}
```

- [ ] **Step 2: Register handler in main.rs**

Extend the `invoke_handler` list in `src-tauri/src/main.rs`:

```rust
            ipc_cmd::verify_node,
```

(Add after the `remove_command` line added in Task 2.)

- [ ] **Step 3: Build to verify compilation**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: compile succeeds.

- [ ] **Step 4: Run test suite**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all existing tests pass (no new test — verify_node is covered by manual smoke).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ipc/commands.rs src-tauri/src/main.rs
git commit -m "ipc: verify_node — spawn az <kind> show, classify exists/missing"
```

---

### Task 4: `execute_node` Tauri command + session execute_lock

**Files:**
- Modify: `src-tauri/src/ipc/state.rs`
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add `execute_lock` to Session**

Replace the `Session` struct in `src-tauri/src/ipc/state.rs` with:

```rust
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use crate::model::Graph;
use crate::parser::ArgMap;

pub struct Session {
    pub graph: Mutex<Graph>,
    pub argmap: ArgMap,
    pub project_path: Mutex<Option<PathBuf>>,
    /// Held for the duration of a per-node execute. Serializes them.
    pub execute_lock: AsyncMutex<()>,
}

pub type SessionState = Arc<Session>;

impl Session {
    pub fn new(argmap: ArgMap) -> Self {
        Self {
            graph: Mutex::new(Graph::new()),
            argmap,
            project_path: Mutex::new(None),
            execute_lock: AsyncMutex::new(()),
        }
    }
}
```

- [ ] **Step 2: Implement `execute_node`**

Append to `src-tauri/src/ipc/commands.rs` (after `verify_node`):

```rust
#[tauri::command]
pub async fn execute_node(
    logical_key: String,
    app: AppHandle,
    state: tauri::State<'_, SessionState>,
) -> Result<(), String> {
    use crate::model::{NodeId, NodeStatus};
    use crate::runner::{spawn_az, AzConfig, AzEvent};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};
    use chrono::Utc;

    let session = state.inner().clone();
    // Try to acquire the execute lock without waiting. Fail fast if busy.
    let _guard = session.execute_lock.try_lock()
        .map_err(|_| "another command is already executing".to_string())?;

    let node_id = NodeId::from_key(&logical_key)
        .ok_or_else(|| format!("bad logical key: {logical_key}"))?;

    // Find the command tokens belonging to this node.
    let argv: Vec<String> = {
        let g = session.graph.lock().map_err(|e| e.to_string())?;
        let node = g.node(&node_id).ok_or_else(|| format!("node not found: {logical_key}"))?;
        let cmd_id = node.command_id.clone().ok_or_else(|| "node has no command".to_string())?;
        let cmd = g.commands().find(|c| c.id == cmd_id).ok_or_else(|| "command missing".to_string())?;
        cmd.tokens.clone()
    };

    // Mark as Running (pid placeholder 0 — spawn_az doesn't expose pid via AzEvent today).
    {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(&node_id) {
            n.status = NodeStatus::Running { pid: 0, started_at: Utc::now() };
        }
    }
    let _ = app.emit_all("run-event", serde_json::json!({
        "type": "node-started", "node": node_id.display(), "argv": argv.clone(),
    }));

    let cfg = AzConfig::default();
    let (tx, mut rx) = mpsc::channel::<AzEvent>(64);
    let (_cancel_tx, cancel_rx) = oneshot::channel();
    let node_disp = node_id.display();
    let app_clone = app.clone();
    let argv_spawn = argv.clone();

    // Run spawn_az inline; events drained below.
    let spawn_fut = async move {
        spawn_az(&cfg, &argv_spawn, tx, cancel_rx).await;
    };

    let drain_fut = async {
        let mut exit_code: Option<i32> = None;
        let mut stderr_tail = String::new();
        let mut duration_ms: u64 = 0;
        while let Some(ev) = rx.recv().await {
            match ev {
                AzEvent::Stdout(line) => {
                    let _ = app_clone.emit_all("run-event", serde_json::json!({
                        "type": "node-log", "node": node_disp, "line": line, "is_err": false,
                    }));
                }
                AzEvent::Stderr(line) => {
                    if stderr_tail.len() < 2048 {
                        stderr_tail.push_str(&line);
                        stderr_tail.push('\n');
                    }
                    let _ = app_clone.emit_all("run-event", serde_json::json!({
                        "type": "node-log", "node": node_disp, "line": line, "is_err": true,
                    }));
                }
                AzEvent::Exit { code, duration_ms: d } => { exit_code = Some(code); duration_ms = d; break; }
                AzEvent::Timeout => break,
                AzEvent::Canceled => break,
            }
        }
        (exit_code, stderr_tail, duration_ms)
    };

    let (_, (exit_code, stderr_tail, duration_ms)) = tokio::join!(spawn_fut, drain_fut);

    let status = match exit_code {
        Some(0) => NodeStatus::Succeeded { duration_ms },
        Some(code) => NodeStatus::Failed { exit_code: code, stderr_tail: stderr_tail.clone(), duration_ms },
        None => NodeStatus::Failed { exit_code: -1, stderr_tail: "no exit event".into(), duration_ms: 0 },
    };
    let status_str = match &status {
        NodeStatus::Succeeded { .. } => "succeeded",
        NodeStatus::Failed { .. } => "failed",
        _ => "other",
    };
    {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(&node_id) { n.status = status.clone(); }
    }
    let _ = app.emit_all("run-event", serde_json::json!({
        "type": "node-finished", "node": node_disp, "status": status_str,
    }));
    let _ = app.emit_all("run-event", serde_json::json!({
        "type": "done", "succeeded": if matches!(status, NodeStatus::Succeeded { .. }) { 1 } else { 0 },
                        "failed":    if matches!(status, NodeStatus::Failed { .. }) { 1 } else { 0 },
    }));

    Ok(())
}
```

- [ ] **Step 3: Register handler in main.rs**

Extend the `invoke_handler` list:

```rust
            ipc_cmd::execute_node,
```

- [ ] **Step 4: Build to verify compilation**

```bash
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: compile succeeds.

- [ ] **Step 5: Run test suite**

```bash
cd src-tauri && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/ipc/state.rs src-tauri/src/ipc/commands.rs src-tauri/src/main.rs
git commit -m "ipc: execute_node — single-command run via spawn_az, status + run-event stream"
```

---

## Phase 3: Frontend — stores and IPC wrappers

### Task 5: New stores + IPC wrappers

**Files:**
- Modify: `ui/src/lib/store.ts`
- Modify: `ui/src/lib/ipc.ts`

- [ ] **Step 1: Add stores**

Append to `ui/src/lib/store.ts`:

```ts
export interface ContextMenuState {
  logicalKey: string;
  commandId: string | null;
  origin: "Declared" | "Ghost";
  status: string;
  x: number;
  y: number;
}

export const contextMenu = writable<ContextMenuState | null>(null);
export const lastError = writable<string | null>(null);
```

- [ ] **Step 2: Add IPC wrappers**

Replace the `ipc` block in `ui/src/lib/ipc.ts`:

```ts
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { GraphSnapshot, RunEvent, NodeStatus } from "./types";
import { lastError } from "./store";

async function withErrorStore<T>(p: Promise<T>): Promise<T> {
  try { lastError.set(null); return await p; }
  catch (e) { lastError.set(String(e)); throw e; }
}

export const ipc = {
  addCommand: (line: string) => invoke<string>("add_command", { line }),
  snapshot: () => invoke<GraphSnapshot>("snapshot"),
  dryRun: () => invoke<string[][]>("dry_run"),
  emitScript: (path: string, flavor: "bash" | "powershell") =>
    invoke<void>("emit_script", { args: { path, flavor } }),
  openProject: (path: string) => invoke<GraphSnapshot>("open_project", { path }),
  saveProjectAs: (path: string) => invoke<void>("save_project_as", { path }),
  runLive: () => invoke<void>("run_live"),
  removeCommand: (id: string) =>
    withErrorStore(invoke<void>("remove_command", { id })),
  verifyNode: (logicalKey: string) =>
    withErrorStore(invoke<NodeStatus>("verify_node", { logicalKey })),
  executeNode: (logicalKey: string) =>
    withErrorStore(invoke<void>("execute_node", { logicalKey })),
};

export const onRunEvent = (cb: (ev: RunEvent) => void) =>
  listen<RunEvent>("run-event", e => cb(e.payload));
```

- [ ] **Step 3: Build to verify TypeScript**

```bash
cd ui && npm run build 2>&1 | tail -8
```

Expected: svelte-check passes, vite build succeeds.

- [ ] **Step 4: Commit**

```bash
git add ui/src/lib/store.ts ui/src/lib/ipc.ts
git commit -m "ui(ipc): add contextMenu+lastError stores and remove/verify/execute wrappers"
```

---

## Phase 4: Frontend — UI components

### Task 6: GraphCanvas — cxttap handler + new style rules

**Files:**
- Modify: `ui/src/components/GraphCanvas.svelte`

- [ ] **Step 1: Add cxttap handler**

In `ui/src/components/GraphCanvas.svelte`, change the existing `onMount` block to also handle right-clicks. Locate the `cy.on("tap", ...)` line near the end of `onMount` and replace it with:

```ts
    cy.on("tap", "node[kind]", (ev) => {
      const logical = ev.target.data("logicalKey") as string;
      selectedNodeKey.set(logical);
    });

    cy.on("cxttap", "node[kind]", (ev) => {
      const data = ev.target.data();
      // Prevent the webview's own context menu from stealing the event.
      if (ev.originalEvent) ev.originalEvent.preventDefault();
      contextMenu.set({
        logicalKey: data.logicalKey,
        commandId: data.commandId ?? null,
        origin: data.origin,
        status: data.status,
        x: (ev.originalEvent as MouseEvent)?.clientX ?? 100,
        y: (ev.originalEvent as MouseEvent)?.clientY ?? 100,
      });
    });
```

Add `contextMenu` to the imports at the top of the `<script>` block:

```ts
  import { nodes, edges, selectedNodeKey, contextMenu } from "../lib/store";
```

- [ ] **Step 2: Plumb `commandId` into visual node data**

In the `buildElements` function, the visual-node `data` object is constructed in two branches (multi-prefix VNet expansion and the `else` branch for other nodes). Add `commandId: n.command_id ?? null` to each `data` literal:

In the multi-prefix VNet branch:

```ts
          visualNodes.push({
            data: {
              id: vnetVisualId(key, i),
              logicalKey: key,
              commandId: n.command_id ?? null,
              parent,
              kind: n.kind,
              name: n.name,
              origin: n.origin,
              status: n.status.kind,
              cidr: p,
              range: cidrToRange(p) ? `${cidrToRange(p)!.first} – ${cidrToRange(p)!.last}` : undefined,
            },
          });
```

In the `else` branch:

```ts
        visualNodes.push({
          data: {
            id: key,
            logicalKey: key,
            commandId: n.command_id ?? null,
            parent,
            kind: n.kind,
            name: n.name,
            origin: n.origin,
            status: n.status.kind,
            cidr,
            range: cidr && cidrToRange(cidr) ? `${cidrToRange(cidr)!.first} – ${cidrToRange(cidr)!.last}` : undefined,
          },
        });
```

And update the `VisualNode` interface declaration in the same file:

```ts
  interface VisualNode {
    data: {
      id: string;
      logicalKey: string;
      commandId: string | null;
      parent?: string;
      kind: NodeKind;
      name: string;
      origin: string;
      status: string;
      cidr?: string;
      range?: string;
    };
  }
```

- [ ] **Step 3: Add style rules for verification statuses**

In the `style:` array within the `cytoscape({...})` call, add these rules after the existing `status = 'failed'` rule (and before the `node.rg` rule):

```ts
        { selector: "node[status = 'missing']", style: { "border-color": "#ff8c1a", "border-style": "dashed" } as any },
        { selector: "node[status = 'exists']",  style: { "border-color": "#2a8f3d" } as any },
        { selector: "node[status = 'verifying']", style: { "border-color": "#b58022" } as any },
```

- [ ] **Step 4: Build to verify**

```bash
cd ui && npm run build 2>&1 | tail -8
```

Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
git add ui/src/components/GraphCanvas.svelte
git commit -m "ui(graph): cxttap opens context menu; verification-status border rules"
```

---

### Task 7: `NodeContextMenu.svelte` popup component

**Files:**
- Create: `ui/src/components/NodeContextMenu.svelte`

- [ ] **Step 1: Create the component**

Create `ui/src/components/NodeContextMenu.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { contextMenu, nodes, edges } from "../lib/store";
  import { ipc } from "../lib/ipc";
  import type { ContextMenuState } from "../lib/store";

  let menuEl: HTMLDivElement;
  let x = 0, y = 0;

  $: state = $contextMenu;

  async function refreshSnapshot() {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes);
    edges.set(snap.edges);
  }

  function close() { contextMenu.set(null); }

  async function remove() {
    if (!state?.commandId) return;
    try { await ipc.removeCommand(state.commandId); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  async function verify() {
    if (!state) return;
    try { await ipc.verifyNode(state.logicalKey); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  async function execute() {
    if (!state) return;
    try { await ipc.executeNode(state.logicalKey); await refreshSnapshot(); }
    catch { /* lastError store set by wrapper */ }
    finally { close(); }
  }

  function isRunning(s: string): boolean { return s === "running"; }
  function isVerifying(s: string): boolean { return s === "verifying"; }

  $: isDeclared = state?.origin === "Declared";
  $: isGhost = state?.origin === "Ghost";
  $: showRemove = isDeclared && state !== null;
  $: showVerify = state !== null;
  $: showExecute = isDeclared && state !== null;
  $: removeDisabled = !!state && (isRunning(state.status) || !state.commandId);
  $: verifyDisabled = !!state && (isRunning(state.status) || isVerifying(state.status));
  $: executeDisabled = !!state && isRunning(state.status);

  // Viewport clamp (rough: 180×140 budget).
  $: if (state) {
    x = Math.min(state.x, window.innerWidth  - 180);
    y = Math.min(state.y, window.innerHeight - 140);
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === "Escape" && state) close();
  }

  onMount(() => { window.addEventListener("keydown", onKey); });
  onDestroy(() => { window.removeEventListener("keydown", onKey); });
</script>

{#if state}
  <div class="ctx-overlay" on:click={close} role="presentation"></div>
  <div class="ctx-menu" style="left:{x}px; top:{y}px;" bind:this={menuEl}>
    {#if showRemove}
      <button class="ctx-item ctx-destructive" on:click={remove} disabled={removeDisabled}>Remove</button>
    {/if}
    {#if showVerify}
      <button class="ctx-item" on:click={verify} disabled={verifyDisabled}>Check Azure</button>
    {/if}
    {#if showExecute}
      <button class="ctx-item" on:click={execute} disabled={executeDisabled}>Execute</button>
    {/if}
  </div>
{/if}

<style>
  .ctx-overlay {
    position: fixed; inset: 0; z-index: 999;
    background: transparent;
  }
  .ctx-menu {
    position: fixed; z-index: 1000;
    background: #ffffff;
    border: 1px solid #ddd; border-radius: 6px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
    min-width: 140px;
    padding: 4px 0;
    font-family: system-ui, sans-serif;
  }
  .ctx-item {
    display: block; width: 100%;
    padding: 6px 14px; margin: 0;
    background: transparent; border: 0;
    text-align: left; font-size: 12px; color: #222;
    cursor: pointer;
  }
  .ctx-item:hover:not([disabled]) { background: #f0f4fa; }
  .ctx-destructive { color: #b53030; }
  .ctx-item[disabled] { opacity: 0.45; cursor: default; }
</style>
```

- [ ] **Step 2: Build to verify TypeScript + Svelte**

```bash
cd ui && npm run build 2>&1 | tail -8
```

Expected: build succeeds with 0 errors.

- [ ] **Step 3: Commit**

```bash
git add ui/src/components/NodeContextMenu.svelte
git commit -m "ui: NodeContextMenu component — Remove/Check Azure/Execute actions"
```

---

### Task 8: Mount NodeContextMenu in App + wire lastError in CommandPane

**Files:**
- Modify: `ui/src/App.svelte`
- Modify: `ui/src/components/CommandPane.svelte`

- [ ] **Step 1: Mount `<NodeContextMenu/>` at app root**

Modify `ui/src/App.svelte` to import and render the new component:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";
  import Toolbar from "./components/Toolbar.svelte";
  import CommandPane from "./components/CommandPane.svelte";
  import GraphCanvas from "./components/GraphCanvas.svelte";
  import DetailPane from "./components/DetailPane.svelte";
  import LogPane from "./components/LogPane.svelte";
  import NodeContextMenu from "./components/NodeContextMenu.svelte";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<div class="app">
  <Toolbar />
  <div class="grid">
    <CommandPane />
    <div class="canvas-wrap"><GraphCanvas /></div>
    <div class="right">
      <DetailPane />
      <div class="divider" />
      <LogPane />
    </div>
  </div>
</div>
<NodeContextMenu />

<style>
  :global(html, body, #app) { height:100%; margin:0; }
  .app { display:flex; flex-direction:column; height:100vh; font-family:system-ui, sans-serif; }
  .grid { flex:1; display:grid; grid-template-columns: 280px 1fr 300px; min-height:0; }
  .canvas-wrap { background:#fff; border-left:1px solid #ddd; border-right:1px solid #ddd; min-height:0; }
  .right { display:grid; grid-template-rows: auto 1px 1fr; background:#fafafa; min-height:0; }
  .divider { background:#ddd; }
</style>
```

- [ ] **Step 2: Wire `lastError` into CommandPane**

Replace `ui/src/components/CommandPane.svelte` with:

```svelte
<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges, lastError } from "../lib/store";
  let line = "";
  let localErr = "";

  async function add() {
    localErr = "";
    try {
      await ipc.addCommand(line.trim());
      line = "";
      const snap = await ipc.snapshot();
      nodes.set(snap.nodes); edges.set(snap.edges);
      lastError.set(null);
    } catch (e) { localErr = String(e); }
  }

  // Display whichever error is most recent — localErr from add-command,
  // or the shared lastError from remove/verify/execute wrappers.
  $: err = localErr || $lastError || "";
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
```

- [ ] **Step 3: Build to verify**

```bash
cd ui && npm run build 2>&1 | tail -8
```

Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add ui/src/App.svelte ui/src/components/CommandPane.svelte
git commit -m "ui: mount NodeContextMenu; wire lastError into CommandPane"
```

---

## Phase 5: Verification

### Task 9: Manual smoke test via Tauri dev

**Files:** none (runtime verification only)

- [ ] **Step 1: Launch dev app**

In a separate terminal from the repo root:

```bash
cd D:\AI\projects\az-plotter
cargo tauri dev
```

- [ ] **Step 2: Verify right-click shows menu**

In the CommandPane, add:

```
az network vnet create -g lakeflow -n net-hub --address-prefixes 10.0.0.0/26
az network vnet subnet create -g lakeflow -n snet-app --vnet-name net-hub --address-prefixes 10.0.0.0/27
az network vnet subnet create -g lakeflow -n gw --vnet-name missing-vnet --address-prefixes 10.0.0.32/27
```

The third command creates a ghost `vnet/missing-vnet`.

Checklist:

- [ ] Right-click the `net-hub` VNet node → menu shows **Remove** (red), **Check Azure**, **Execute**. Close with Escape.
- [ ] Right-click `snet-app` → menu shows the same three items.
- [ ] Right-click the ghost `missing-vnet` node → menu shows ONLY **Check Azure** (ghost has no command).
- [ ] Click outside the menu → it closes.

- [ ] **Step 3: Verify Remove with and without dependents**

- [ ] Right-click `net-hub` VNet → **Remove** → expect error in CommandPane's red err line like `Can't remove vnet/net-hub@rg:lakeflow: subnet/snet-app@rg:lakeflow depends on it. Remove dependents first.` Graph unchanged.
- [ ] Right-click `snet-app` → **Remove** → subnet disappears.
- [ ] Right-click `net-hub` → **Remove** → VNet disappears.
- [ ] Right-click `gw` (the subnet referencing the ghost VNet) → **Remove** → both `gw` and the ghost `missing-vnet` disappear.

- [ ] **Step 4: Verify Check Azure (requires az binary on PATH)**

With `az` installed and logged in:

- [ ] Create a command that matches a real resource you own → right-click → **Check Azure** → border turns green.
- [ ] Create a command for a name that definitely doesn't exist → **Check Azure** → border turns orange dashed.
- [ ] If `az` is not installed, the action should fail with `.err` = `az not found: ...` or similar.

- [ ] **Step 5: Verify Execute**

Preferred: use the `fake-az` helper built from `src-fake-az` to avoid touching real Azure:

```bash
# In a separate terminal, build fake-az into a dir on PATH (or export AZ_FAKE_SCRIPT once cargo tauri dev is running)
cd src-fake-az && cargo build
```

With `AZ_FAKE_SCRIPT='[{ "stdout": "ok\n", "exit_code": 0 }]'` in the environment the Tauri process inherits (best set before `cargo tauri dev`):

- [ ] Right-click any declared node → **Execute** → node flashes amber, then green on success. LogPane shows `[vnet/...] ok`.
- [ ] Change `AZ_FAKE_SCRIPT` to `'[{ "stderr": "boom\n", "exit_code": 1 }]'`, restart dev, re-execute → node turns red, `.err` shows the stderr tail.
- [ ] While one execute is in flight (use a script with `"delay_ms": 3000`), right-click another declared node → Execute action should either be disabled visually (if you're fast) or error with `.err` = `another command is already executing`.

- [ ] **Step 6: Commit nothing; mark plan complete**

No code changes in this task if all checks pass. If a check reveals a bug, fix it inline, retest, commit, and note the fix.

```bash
git log --oneline | head -10
```

Expected: ≥ 8 new commits since the spec commit (`49a6016`).

---

## Spec coverage check

| Spec section | Implementation task |
|---|---|
| §2 Goals — Remove, Check Azure, Execute | Tasks 2, 3, 4 (backend), 5–8 (frontend) |
| §4 `cxttap` trigger | Task 6 |
| §4 Custom Svelte popup (not cxtmenu) | Task 7 |
| §4 Mouse coords + viewport clamp | Task 6 (coords), Task 7 (clamp) |
| §4 Dismiss via outside/Escape/action | Task 7 |
| §4 Multi-prefix VNet = single logical action | Tasks 6 (commandId from visual data → backend looks up single command) |
| §4 Dep-refusal with first dependent named | Task 2 (`do_remove_command` dependents loop + error message) |
| §4 Ghost cleanup rule | Task 2 (refs iteration after removal) |
| §4 Check Azure: 10 s timeout, exit → status | Task 3 (`do_verify_node` + `run_and_classify`) |
| §4 Execute: tokens-based spawn_az, run-event stream, re-run OK | Task 4 |
| §4 Execute serialize via session mutex | Task 4 (execute_lock + try_lock) |
| §4 Error display in CommandPane `.err` | Task 5 (lastError store) + Task 8 (wiring) |
| §5.2 Files touched mapping | Each task lists exact files |
| §5.3 Types | Task 5 (TS), Task 4 (Rust `execute_lock`) |
| §5.4 Menu availability per origin × status | Task 7 (show/disabled predicates) |
| §6 New Cytoscape style rules | Task 6 (Step 3) |
| §6 Menu markup + clamp | Task 7 |
| §7 Error behavior table | Tasks 2, 3, 4 (backend returns Err), Task 5 (wrapper sets lastError), Task 8 (display) |
| §8 Backend unit tests | Task 1 (graph + from_key) |
| §8 Backend integration tests | Task 2 (context_menu.rs) |
| §8 Frontend unit tests | None — covered by manual smoke per spec |
| §8 Manual smoke | Task 9 |

---

## Execution options

Plan complete and saved to `docs/superpowers/plans/2026-04-19-node-context-menu-plan.md`.

Two ways to execute:

**1. Subagent-Driven (recommended)** — Fresh subagent per task, review between tasks.

**2. Inline Execution** — All tasks in this session with checkpoints.

Which approach?
