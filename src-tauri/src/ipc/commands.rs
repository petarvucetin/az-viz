use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::model::{Edge, Node};
use crate::parser::{commit as commit_parse, parse};
use crate::persist::ProjectFile;
use crate::runner::{dry_run as runner_dry_run, write_script, ScriptFlavor};
use crate::runner::{live_run, AzConfig, RunEvent};
use super::state::SessionState;

#[derive(Serialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[tauri::command]
pub fn add_command(line: String, state: tauri::State<SessionState>) -> Result<String, String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    let parsed = parse(&line, &state.argmap, &g).map_err(|e| e.to_string())?;
    let id = parsed.command.id.clone();
    commit_parse(&mut g, parsed).map_err(|e| e.to_string())?;
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(id)
}

#[tauri::command]
pub fn snapshot(state: tauri::State<SessionState>) -> Result<GraphSnapshot, String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    Ok(GraphSnapshot {
        nodes: g.nodes().cloned().collect(),
        edges: g.edges().cloned().collect(),
    })
}

#[tauri::command]
pub fn dry_run(state: tauri::State<SessionState>) -> Result<Vec<Vec<String>>, String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let plan = runner_dry_run(&g).map_err(|e| e.to_string())?;
    Ok(plan.into_iter().map(|c| c.argv).collect())
}

#[derive(Deserialize)]
pub struct EmitArgs { pub path: String, pub flavor: String }

#[tauri::command]
pub fn emit_script(args: EmitArgs, state: tauri::State<SessionState>) -> Result<(), String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let plan = runner_dry_run(&g).map_err(|e| e.to_string())?;
    let flavor = match args.flavor.as_str() {
        "bash" => ScriptFlavor::Bash,
        "powershell" => ScriptFlavor::Powershell,
        _ => return Err(format!("unknown flavor: {}", args.flavor)),
    };
    let source = state.project_path.lock().map_err(|e| e.to_string())?
        .as_ref().map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "<untitled>".into());
    write_script(&plan, flavor, &source, &PathBuf::from(&args.path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_project(path: String, state: tauri::State<SessionState>) -> Result<GraphSnapshot, String> {
    let p = PathBuf::from(&path);
    let pf = ProjectFile::load(&p).map_err(|e| e.to_string())?;
    let g = pf.to_graph(&state.argmap).map_err(|e| e.to_string())?;
    let nodes: Vec<Node> = g.nodes().cloned().collect();
    let edges: Vec<Edge> = g.edges().cloned().collect();
    *state.graph.lock().map_err(|e| e.to_string())? = g;
    *state.project_path.lock().map_err(|e| e.to_string())? = Some(p);
    Ok(GraphSnapshot { nodes, edges })
}

#[tauri::command]
pub fn save_project_as(path: String, state: tauri::State<SessionState>) -> Result<(), String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let pf = ProjectFile::from_graph(&g);
    let p = PathBuf::from(&path);
    pf.save(&p).map_err(|e| e.to_string())?;
    *state.project_path.lock().map_err(|e| e.to_string())? = Some(p);
    Ok(())
}

#[tauri::command]
pub async fn run_live(app: AppHandle, state: tauri::State<'_, SessionState>) -> Result<(), String> {
    let graph = {
        let g = state.graph.lock().map_err(|e| e.to_string())?;
        g.clone()
    };
    let cfg = AzConfig::default();
    let mut handle = live_run(&graph, cfg).await.map_err(|e| e.to_string())?;
    while let Some(ev) = handle.events.recv().await {
        let is_done = matches!(ev, RunEvent::Done { .. });
        let payload = serde_json::to_value(&RunEventWire::from(&ev)).unwrap();
        let _ = app.emit_all("run-event", payload);
        if is_done { break; }
    }
    Ok(())
}

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
        .filter(|to_id| g.node(*to_id).map(|n| matches!(n.origin, crate::model::Origin::Declared)).unwrap_or(false))
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

    // Autosave if a project is open. Matches add_command's lock ordering
    // (graph → project_path) to prevent deadlock under concurrent IPC.
    if let Some(path) = session.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = crate::persist::ProjectFile::from_graph(&g).save(path);
    }
    Ok(())
}

#[tauri::command]
pub fn remove_command(id: String, state: tauri::State<SessionState>) -> Result<(), String> {
    do_remove_command(&id, state.inner())
}

/// Run `az <kind> show --name N --resource-group RG [--subscription S]` and update
/// `node.status` to `Exists` or `Missing` based on exit code. 10 s timeout.
pub async fn do_verify_node(
    logical_key: &str,
    session: &crate::ipc::state::Session,
) -> Result<crate::model::NodeStatus, String> {
    use crate::model::{NodeId, NodeKind, NodeStatus};

    let node_id = NodeId::from_key(logical_key)
        .ok_or_else(|| format!("bad logical key: {logical_key}"))?;

    // Acquire graph once: set status = Verifying AND look up any needed parent name.
    let parent_name: Option<String> = {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        match g.node_mut(&node_id) {
            Some(n) => n.status = NodeStatus::Verifying,
            None => return Err(format!("node not found: {logical_key}")),
        }
        match node_id.kind {
            NodeKind::Subnet => {
                let parent = g.parents(&node_id).find_map(|p| {
                    if matches!(p.kind, NodeKind::Vnet) {
                        Some(p.name.clone())
                    } else { None }
                });
                match parent {
                    Some(n) => Some(n),
                    None => return Err(format!("subnet {} has no parent VNet in the graph", logical_key)),
                }
            }
            NodeKind::NsgRule => {
                let parent = g.parents(&node_id).find_map(|p| {
                    if matches!(p.kind, NodeKind::Nsg) {
                        Some(p.name.clone())
                    } else { None }
                });
                match parent {
                    Some(n) => Some(n),
                    None => return Err(format!("nsg rule {} has no parent NSG in the graph", logical_key)),
                }
            }
            NodeKind::VnetPeering => {
                let parent = g.parents(&node_id).find_map(|p| {
                    if matches!(p.kind, NodeKind::Vnet) { Some(p.name.clone()) } else { None }
                });
                match parent {
                    Some(n) => Some(n),
                    None => return Err(format!("vnet peering {} has no parent VNet in the graph", logical_key)),
                }
            }
            _ => None,
        }
    };

    // Build the az argv.
    let argv: Vec<String> = match node_id.kind {
        NodeKind::ResourceGroup => {
            let mut a = vec!["group".to_string(), "show".into(), "--name".into(), node_id.name.clone()];
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        NodeKind::Subnet => {
            let vnet = parent_name.as_ref().expect("subnet parent checked above");
            let mut a: Vec<String> = ["network", "vnet", "subnet", "show"].iter().map(|s| s.to_string()).collect();
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
                "--vnet-name".into(), vnet.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        NodeKind::NsgRule => {
            let nsg = parent_name.as_ref().expect("nsg-rule parent checked above");
            let mut a: Vec<String> = ["network", "nsg", "rule", "show"].iter().map(|s| s.to_string()).collect();
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
                "--nsg-name".into(), nsg.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        NodeKind::DnsResolver => {
            let mut a: Vec<String> = vec!["dns-resolver".into(), "show".into()];
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        NodeKind::VnetPeering => {
            let vnet = parent_name.as_ref().expect("vnet-peering parent checked above");
            let mut a: Vec<String> = ["network", "vnet", "peering", "show"].iter().map(|s| s.to_string()).collect();
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
                "--vnet-name".into(), vnet.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        other => {
            let kind_str = match other {
                NodeKind::Vnet => "vnet",
                NodeKind::Nsg => "nsg",
                NodeKind::PublicIp => "public-ip",
                NodeKind::Nic => "nic",
                NodeKind::Lb => "lb",
                NodeKind::RouteTable => "route-table",
                NodeKind::VnetGateway => "vnet-gateway",
                NodeKind::LocalGateway => "local-gateway",
                NodeKind::VpnConnection => "vpn-connection",
                _ => unreachable!(),
            };
            let mut a: Vec<String> = vec!["network".into(), kind_str.into(), "show".into()];
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
    };

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

    // Drain events; capture stderr's first line so we can distinguish a spawn
    // failure (stderr starts with "spawn error:" and exit is -1 at duration 0)
    // from a genuine non-zero az exit.
    let mut exit_code: Option<i32> = None;
    let mut exit_duration_ms: u64 = 0;
    let mut stderr_first: Option<String> = None;
    let mut timed_out = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            AzEvent::Stderr(line) if stderr_first.is_none() => { stderr_first = Some(line); }
            AzEvent::Exit { code, duration_ms } => { exit_code = Some(code); exit_duration_ms = duration_ms; break; }
            AzEvent::Timeout => { timed_out = true; break; }
            _ => {}
        }
    }

    // Helper: compare-and-swap the status back to a target ONLY if still Verifying.
    // Prevents clobbering a concurrent execute_node that may have advanced the
    // node to Running in the meantime.
    let cas_status = |target: NodeStatus| -> Result<(), String> {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(node_id) {
            if matches!(n.status, NodeStatus::Verifying) {
                n.status = target;
            }
        }
        Ok(())
    };

    // Spawn-failure detection: az.rs emits Exit { code: -1, duration_ms: 0 }
    // preceded by Stderr("spawn error: ...") when Command::spawn fails.
    let is_spawn_error = exit_code == Some(-1)
        && exit_duration_ms == 0
        && stderr_first.as_deref().map_or(false, |s| s.starts_with("spawn error:"));

    if timed_out {
        cas_status(NodeStatus::Unverified)?;
        return Err("verify timed out".into());
    }
    if is_spawn_error {
        cas_status(NodeStatus::Unverified)?;
        return Err(stderr_first.unwrap_or_else(|| "spawn error".into()));
    }

    let new_status = if exit_code == Some(0) {
        NodeStatus::Exists
    } else {
        NodeStatus::Missing
    };
    cas_status(new_status.clone())?;
    Ok(new_status)
}

#[tauri::command]
pub async fn verify_node(
    logical_key: String,
    state: tauri::State<'_, SessionState>,
) -> Result<crate::model::NodeStatus, String> {
    do_verify_node(&logical_key, state.inner()).await
}

#[tauri::command]
pub async fn execute_node(
    logical_key: String,
    app: AppHandle,
    state: tauri::State<'_, SessionState>,
) -> Result<(), String> {
    use crate::model::{NodeId, NodeStatus};
    use crate::runner::{spawn_az, AzConfig, AzEvent};
    use std::collections::VecDeque;
    use std::time::Instant;
    use tokio::sync::{mpsc, oneshot};
    use chrono::Utc;

    let session = state.inner().clone();
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

    // Mark as Running.
    {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(&node_id) {
            n.status = NodeStatus::Running { pid: 0, started_at: Utc::now() };
        }
    }
    let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::NodeStarted {
        node: node_id.display(),
        argv: argv.clone(),
    }).unwrap());

    let cfg = AzConfig::default();
    let (tx, mut rx) = mpsc::channel::<AzEvent>(64);
    let (_cancel_tx, cancel_rx) = oneshot::channel();
    let node_disp = node_id.display();
    let app_clone = app.clone();
    let argv_spawn = argv.clone();
    let started_at = Instant::now();

    let spawn_fut = async move {
        spawn_az(&cfg, &argv_spawn, tx, cancel_rx).await;
    };

    // Termination cause discovered while draining the event stream.
    enum Term {
        Exit { code: i32, duration_ms: u64 },
        Timeout,
        Canceled,
        Closed,  // channel closed without an explicit terminator (defensive)
    }

    let drain_fut = async {
        // Bounded deque of recent stderr lines; trim by total byte count.
        let mut stderr_lines: VecDeque<String> = VecDeque::new();
        let mut stderr_bytes: usize = 0;
        const STDERR_BUDGET: usize = 2048;

        let mut term: Term = Term::Closed;
        while let Some(ev) = rx.recv().await {
            match ev {
                AzEvent::Stdout(line) => {
                    let _ = app_clone.emit_all("run-event", serde_json::to_value(&RunEventWire::NodeLog {
                        node: node_disp.clone(), line: line.clone(), is_err: false,
                    }).unwrap());
                }
                AzEvent::Stderr(line) => {
                    // Track tail-bounded stderr.
                    stderr_bytes += line.len() + 1; // +1 for newline join later
                    stderr_lines.push_back(line.clone());
                    while stderr_bytes > STDERR_BUDGET && stderr_lines.len() > 1 {
                        if let Some(dropped) = stderr_lines.pop_front() {
                            stderr_bytes = stderr_bytes.saturating_sub(dropped.len() + 1);
                        }
                    }
                    let _ = app_clone.emit_all("run-event", serde_json::to_value(&RunEventWire::NodeLog {
                        node: node_disp.clone(), line, is_err: true,
                    }).unwrap());
                }
                AzEvent::Exit { code, duration_ms } => { term = Term::Exit { code, duration_ms }; break; }
                AzEvent::Timeout => { term = Term::Timeout; break; }
                AzEvent::Canceled => { term = Term::Canceled; break; }
            }
        }
        let stderr_tail: String = stderr_lines.into_iter().collect::<Vec<_>>().join("\n");
        (term, stderr_tail)
    };

    let (_, (term, stderr_tail)) = tokio::join!(spawn_fut, drain_fut);

    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let status = match term {
        Term::Exit { code: 0, duration_ms } => NodeStatus::Succeeded { duration_ms },
        Term::Exit { code, duration_ms } => NodeStatus::Failed {
            exit_code: code, stderr_tail: stderr_tail.clone(), duration_ms,
        },
        Term::Timeout => NodeStatus::Failed {
            exit_code: -1,
            stderr_tail: if stderr_tail.is_empty() { "timeout".into() } else { format!("timeout\n{stderr_tail}") },
            duration_ms: elapsed_ms,
        },
        Term::Canceled => NodeStatus::Canceled,
        Term::Closed => NodeStatus::Failed {
            exit_code: -1,
            stderr_tail: if stderr_tail.is_empty() { "no exit event".into() } else { stderr_tail.clone() },
            duration_ms: elapsed_ms,
        },
    };
    let status_str = match &status {
        NodeStatus::Succeeded { .. } => "succeeded",
        NodeStatus::Failed { .. } => "failed",
        NodeStatus::Canceled => "canceled",
        _ => "other",
    }.to_string();

    {
        let mut g = session.graph.lock().map_err(|e| e.to_string())?;
        if let Some(n) = g.node_mut(&node_id) { n.status = status.clone(); }
    }
    let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::NodeFinished {
        node: node_disp.clone(), status: status_str,
    }).unwrap());
    let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::Done {
        succeeded: if matches!(status, NodeStatus::Succeeded { .. }) { 1 } else { 0 },
        failed: if matches!(status, NodeStatus::Failed { .. }) { 1 } else { 0 },
    }).unwrap());

    Ok(())
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum RunEventWire {
    NodeStarted { node: String, argv: Vec<String> },
    NodeLog { node: String, line: String, is_err: bool },
    NodeFinished { node: String, status: String },
    Aborted { node: String, reason: String },
    Done { succeeded: usize, failed: usize },
}

impl RunEventWire {
    fn from(ev: &RunEvent) -> Self {
        match ev {
            RunEvent::NodeStarted { node, argv } => Self::NodeStarted { node: node.display(), argv: argv.clone() },
            RunEvent::NodeLog { node, line, is_err } => Self::NodeLog { node: node.display(), line: line.clone(), is_err: *is_err },
            RunEvent::NodeFinished { node, status } => {
                use crate::model::NodeStatus::*;
                let s = match status {
                    Succeeded { .. } => "succeeded",
                    Failed { .. } => "failed",
                    Canceled => "canceled",
                    _ => "other",
                }.to_string();
                Self::NodeFinished { node: node.display(), status: s }
            }
            RunEvent::Aborted { node, reason } => Self::Aborted { node: node.display(), reason: reason.clone() },
            RunEvent::Done { succeeded, failed } => Self::Done { succeeded: *succeeded, failed: *failed },
        }
    }
}
