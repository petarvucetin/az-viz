use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use crate::model::{Edge, Group, Node, Variable};
use crate::parser::{commit as commit_parse, parse_line, ParsedLine};
use crate::persist::ProjectFile;
use crate::runner::{dry_run as runner_dry_run, write_script, ScriptFlavor};
use crate::runner::{live_run, AzConfig, RunEvent};
use super::state::SessionState;

#[derive(Serialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub variables: Vec<Variable>,
    /// Mapping from a command's logical node key → names of variables the
    /// command references. Lets the UI nest a variable under its consumer.
    pub var_consumers: std::collections::BTreeMap<String, Vec<String>>,
    pub groups: Vec<Group>,
    /// Mapping from group id → logical node keys of commands in the group,
    /// in declaration order. Canvas uses this to parent nodes under a group.
    pub group_nodes: std::collections::BTreeMap<String, Vec<String>>,
}

#[tauri::command]
pub fn add_command(line: String, state: tauri::State<SessionState>) -> Result<String, String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    let parsed = parse_line(&line, &state.argmap, &g).map_err(|e| e.to_string())?;
    let id = match parsed {
        ParsedLine::Command(p) => {
            let id = p.command.id.clone();
            commit_parse(&mut g, p).map_err(|e| e.to_string())?;
            id
        }
        ParsedLine::Variable(v) => {
            let name = v.name.clone();
            g.upsert_variable(v);
            format!("var:{name}")
        }
    };
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(id)
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BatchAddResult {
    /// A command (or variable assignment) was added successfully.
    Command { id: String },
    /// A `# <title>` header line was recognized (opens a new section).
    Section { title: String },
    /// The line parsed cleanly but the produced resource was already
    /// declared by an earlier command, so it's a no-op duplicate.
    Duplicate { line_index: usize, line: String, produces: String },
    /// An error occurred parsing a line; subsequent lines are not processed.
    Error { line_index: usize, line: String, message: String },
}

/// Add a batch of lines. Handles `# <title>` lines: a `#` comment opens a
/// new group; the group is finalized only if 2+ commands follow before the
/// next `#` or end-of-input (single-command sections stay ungrouped, per
/// user spec).
///
/// Returns the per-command outcomes in order. Stops at the first parse
/// error; the caller can display it and leave the rest of the input in the
/// textarea for retry.
#[tauri::command]
pub fn add_commands_batch(
    lines: Vec<String>,
    state: tauri::State<SessionState>,
) -> Result<Vec<BatchAddResult>, String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    let mut results: Vec<BatchAddResult> = Vec::new();

    // Pending group = a `#` header has been seen; pending_ids collects the
    // command IDs added while it's active. On group close (next `#` or EOF)
    // we finalize to a real Group only if 2+ commands were added.
    let mut pending_title: Option<String> = None;
    let mut pending_ids: Vec<String> = Vec::new();

    let finalize = |g: &mut crate::model::Graph,
                    title: &Option<String>,
                    ids: &mut Vec<String>| {
        if let Some(t) = title {
            // Any `#` header with at least 1 command becomes a group.
            if ids.len() >= 1 {
                let gid = format!("grp-{}", uuid::Uuid::new_v4());
                g.add_group(Group { id: gid.clone(), title: t.clone(), command_ids: ids.clone() });
                for cid in ids.iter() {
                    if let Some(c) = g.commands_mut().find(|c| &c.id == cid) {
                        c.group_id = Some(gid.clone());
                    }
                }
            }
        }
        ids.clear();
    };

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.is_empty() { continue; }
        if let Some(rest) = line.strip_prefix('#') {
            // Close the previous pending group before starting a new one.
            finalize(&mut g, &pending_title, &mut pending_ids);
            let title = rest.trim().to_string();
            results.push(BatchAddResult::Section { title: title.clone() });
            pending_title = Some(title);
            continue;
        }
        match parse_line(raw, &state.argmap, &g) {
            Ok(ParsedLine::Command(p)) => {
                let id = p.command.id.clone();
                let produces_id = p.command.produces.clone();
                // Detect "no-op duplicate": the produced node already exists
                // and is already Declared by a prior command. Commit would
                // silently merge props, which the user reads as "nothing
                // happened". Skip the commit and return a Duplicate result.
                let already_declared = g.node(&produces_id)
                    .map(|n| matches!(n.origin, crate::model::Origin::Declared))
                    .unwrap_or(false);
                if already_declared {
                    results.push(BatchAddResult::Duplicate {
                        line_index: i, line: raw.clone(), produces: produces_id.display(),
                    });
                    continue;
                }
                if let Err(e) = commit_parse(&mut g, p) {
                    results.push(BatchAddResult::Error {
                        line_index: i, line: raw.clone(), message: e.to_string(),
                    });
                    break;
                }
                if pending_title.is_some() { pending_ids.push(id.clone()); }
                results.push(BatchAddResult::Command { id });
            }
            Ok(ParsedLine::Variable(v)) => {
                let name = v.name.clone();
                g.upsert_variable(v);
                results.push(BatchAddResult::Command { id: format!("var:{name}") });
            }
            Err(e) => {
                results.push(BatchAddResult::Error {
                    line_index: i, line: raw.clone(), message: e.to_string(),
                });
                break;
            }
        }
    }
    finalize(&mut g, &pending_title, &mut pending_ids);

    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(results)
}

#[tauri::command]
pub fn snapshot(state: tauri::State<SessionState>) -> Result<GraphSnapshot, String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    // Return nodes in command-insertion order (first command that produced each
    // node wins its position), with orphaned ghost nodes appended at the end.
    let mut seen: std::collections::HashSet<crate::model::NodeId> = std::collections::HashSet::new();
    let mut ordered: Vec<Node> = Vec::new();
    for c in g.commands() {
        if let Some(n) = g.node(&c.produces) {
            if seen.insert(n.id.clone()) {
                ordered.push(n.clone());
            }
        }
    }
    for n in g.nodes() {
        if seen.insert(n.id.clone()) {
            ordered.push(n.clone());
        }
    }
    // Build consumer map: logical key of produced node → sorted var_refs.
    let mut var_consumers: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for c in g.commands() {
        if c.var_refs.is_empty() { continue; }
        let key = c.produces.display();
        var_consumers.entry(key).or_default().extend(c.var_refs.iter().cloned());
    }
    let group_nodes = build_group_nodes(&g);
    Ok(GraphSnapshot {
        nodes: ordered,
        edges: g.edges().cloned().collect(),
        variables: g.variables().cloned().collect(),
        var_consumers,
        groups: g.groups().cloned().collect(),
        group_nodes,
    })
}

fn build_group_nodes(g: &crate::model::Graph) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for c in g.commands() {
        if let Some(gid) = &c.group_id {
            out.entry(gid.clone()).or_default().push(c.produces.display());
        }
    }
    out
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
    let variables: Vec<Variable> = g.variables().cloned().collect();
    let mut var_consumers: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for c in g.commands() {
        if c.var_refs.is_empty() { continue; }
        let key = c.produces.display();
        var_consumers.entry(key).or_default().extend(c.var_refs.iter().cloned());
    }
    let groups = g.groups().cloned().collect();
    let group_nodes = build_group_nodes(&g);
    *state.graph.lock().map_err(|e| e.to_string())? = g;
    *state.project_path.lock().map_err(|e| e.to_string())? = Some(p);
    Ok(GraphSnapshot { nodes, edges, variables, var_consumers, groups, group_nodes })
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

    // Remove from any group it was a member of. Drop the group if removing
    // this command leaves it with < 2 members (per the "groups need ≥ 2"
    // rule used at creation time).
    let group_id_opt = cmd.group_id.clone();
    if let Some(gid) = group_id_opt {
        if let Some(group) = g.group_mut(&gid) {
            group.command_ids.retain(|c| c != id);
            let remaining = group.command_ids.clone();
            // Drop the group only if it's become empty. Single-member
            // groups are valid (matching the "≥1" rule at creation time).
            if remaining.is_empty() {
                g.remove_group(&gid);
            }
        }
    }

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

#[derive(Deserialize)]
pub struct SetVariableArgs { pub name: String, pub body: String }

/// Set or update a variable's body from a plain string (as typed by the user).
/// If `body` begins with `az ` or `$(az ...)`, it's stored as a command argv;
/// otherwise it's stored as a literal. `Declared` origin + `resolved: None`
/// (forces a re-resolve on next execute).
#[tauri::command]
pub fn set_variable_body(
    args: SetVariableArgs,
    state: tauri::State<SessionState>,
) -> Result<(), String> {
    use crate::model::{VarBody, VarOrigin};
    use crate::parser::varsyntax;
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    let body = varsyntax::body_from_rhs(&args.body);
    // For literal bodies the resolved value is trivially the body itself, so
    // cache it immediately. Command bodies need an explicit Execute to run.
    let resolved = match &body {
        VarBody::Literal { value } => Some(value.clone()),
        _ => None,
    };
    let updated = match g.variable(&args.name) {
        Some(existing) => {
            let mut v = existing.clone();
            v.body = body;
            v.origin = VarOrigin::Declared;
            v.resolved = resolved;
            v
        }
        None => Variable { name: args.name.clone(), body, origin: VarOrigin::Declared, resolved },
    };
    g.upsert_variable(updated);
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(())
}

/// Re-run a variable's producer command (Command body only) and store the
/// trimmed stdout in `resolved`. Emits `login-log`-style events so the user
/// sees the command output in the Log pane. No-op for Literal/Unset bodies.
#[tauri::command]
pub async fn refresh_variable(
    name: String,
    app: AppHandle,
    state: tauri::State<'_, SessionState>,
) -> Result<Option<String>, String> {
    use crate::model::VarBody;
    let session = state.inner().clone();
    let body = {
        let g = session.graph.lock().map_err(|e| e.to_string())?;
        g.variable(&name).ok_or_else(|| format!("variable not found: {name}"))?.body.clone()
    };
    match body {
        VarBody::Unset => Err(format!("variable {name} has no body set")),
        VarBody::Literal { value } => {
            let mut g = session.graph.lock().map_err(|e| e.to_string())?;
            if let Some(v) = g.variable_mut(&name) { v.resolved = Some(value.clone()); }
            Ok(Some(value))
        }
        VarBody::Command { argv } => {
            let resolved = resolve_var_command(&name, &argv, &app).await?;
            let mut g = session.graph.lock().map_err(|e| e.to_string())?;
            if let Some(v) = g.variable_mut(&name) { v.resolved = Some(resolved.clone()); }
            if let Some(path) = session.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
                let _ = ProjectFile::from_graph(&g).save(path);
            }
            Ok(Some(resolved))
        }
    }
}

#[tauri::command]
pub fn remove_variable(name: String, state: tauri::State<SessionState>) -> Result<(), String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    g.remove_variable(&name);
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(())
}

/// Run a variable's producer argv, streaming output as `node-log`-style
/// events tagged with the variable's name, and return the trimmed stdout.
async fn resolve_var_command(
    name: &str,
    argv: &[String],
    app: &AppHandle,
) -> Result<String, String> {
    use crate::runner::{default_az_exe, looks_like_not_logged_in, spawn_az, AzConfig, AzEvent};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    let cfg = AzConfig { exe: default_az_exe().into(), timeout: Duration::from_secs(30) };
    let (tx, mut rx) = mpsc::channel::<AzEvent>(64);
    let (_cancel_tx, cancel_rx) = oneshot::channel();
    let argv_vec = argv.to_vec();

    let node_tag = format!("var:{name}");
    let app_for_drain = app.clone();

    let spawn_fut = async move { spawn_az(&cfg, &argv_vec, tx, cancel_rx).await; };
    let drain_fut = async move {
        let mut stdout_all = String::new();
        let mut stderr_tail = String::new();
        let mut exit: Option<i32> = None;
        let mut timed_out = false;
        while let Some(ev) = rx.recv().await {
            match ev {
                AzEvent::Stdout(line) => {
                    if !stdout_all.is_empty() { stdout_all.push('\n'); }
                    stdout_all.push_str(&line);
                    // Per-line log, same shape as node-log.
                    let _ = app_for_drain.emit_all("run-event", serde_json::to_value(
                        &RunEventWire::NodeLog { node: node_tag.clone(), line, is_err: false }
                    ).unwrap());
                }
                AzEvent::Stderr(line) => {
                    if stderr_tail.len() < 2048 {
                        stderr_tail.push_str(&line);
                        stderr_tail.push('\n');
                    }
                    let _ = app_for_drain.emit_all("run-event", serde_json::to_value(
                        &RunEventWire::NodeLog { node: node_tag.clone(), line, is_err: true }
                    ).unwrap());
                }
                AzEvent::Exit { code, .. } => { exit = Some(code); break; }
                AzEvent::Timeout => { timed_out = true; break; }
                AzEvent::Canceled => break,
            }
        }
        (stdout_all, stderr_tail, exit, timed_out)
    };
    let (_, (stdout_all, stderr_tail, exit, timed_out)) = tokio::join!(spawn_fut, drain_fut);

    if timed_out { return Err(format!("resolving ${name} timed out")); }
    if exit != Some(0) {
        if looks_like_not_logged_in(&stderr_tail) {
            let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::AuthRequired {
                triggered_by: "resolve".to_string(),
                logical_key: format!("var:{name}"),
            }).unwrap());
            return Err("not_logged_in".into());
        }
        return Err(format!(
            "resolving ${name} failed (exit {}): {}",
            exit.unwrap_or(-1), stderr_tail.trim()
        ));
    }
    Ok(stdout_all.trim().to_string())
}

/// Replace the graph with an empty one. Autosaves the empty state to the
/// open project file, if any, matching the persistence behavior of
/// add_command / remove_command.
#[tauri::command]
pub fn clear_all(state: tauri::State<SessionState>) -> Result<(), String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    *g = crate::model::Graph::new();
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(())
}

/// Run `az <kind> show --name N --resource-group RG [--subscription S]` and update
/// `node.status` to `Exists` or `Missing` based on exit code. 10 s timeout.
///
/// If `app` is `Some` and the check fails with an Azure-CLI "not logged in"
/// signature, an `auth-required` run-event is emitted and the node's status is
/// rolled back to `Unverified` instead of being marked `Missing`.
pub async fn do_verify_node(
    logical_key: &str,
    session: &crate::ipc::state::Session,
    app: Option<&AppHandle>,
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
            NodeKind::PrivateDnsLink => {
                let parent = g.parents(&node_id).find_map(|p| {
                    if matches!(p.kind, NodeKind::PrivateDnsZone) { Some(p.name.clone()) } else { None }
                });
                match parent {
                    Some(n) => Some(n),
                    None => return Err(format!("private-dns link {} has no parent private-dns-zone in the graph", logical_key)),
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
        NodeKind::PrivateDnsZone => {
            let mut a: Vec<String> = vec!["network".into(), "private-dns".into(), "zone".into(), "show".into()];
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
            ]);
            if let Some(ref sub) = node_id.subscription { a.extend(["--subscription".into(), sub.clone()]); }
            a
        }
        NodeKind::PrivateDnsLink => {
            let zone = parent_name.as_ref().expect("private-dns-link parent checked above");
            let mut a: Vec<String> = ["network", "private-dns", "link", "vnet", "show"].iter().map(|s| s.to_string()).collect();
            a.extend([
                "--name".into(), node_id.name.clone(),
                "--resource-group".into(), node_id.resource_group.clone(),
                "--zone-name".into(), zone.clone(),
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

    run_and_classify(&node_id, argv, session, app).await
}

async fn run_and_classify(
    node_id: &crate::model::NodeId,
    argv: Vec<String>,
    session: &crate::ipc::state::Session,
    app: Option<&AppHandle>,
) -> Result<crate::model::NodeStatus, String> {
    use crate::model::NodeStatus;
    use crate::runner::{default_az_exe, looks_like_not_logged_in, spawn_az, AzConfig, AzEvent};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    let cfg = AzConfig { exe: default_az_exe().into(), timeout: Duration::from_secs(10) };
    let (tx, mut rx) = mpsc::channel::<AzEvent>(64);
    let (_cancel_tx, cancel_rx) = oneshot::channel();

    // Spawn and drain must run concurrently: `az ... show` can emit JSON
    // larger than the channel buffer, and awaiting spawn_az to completion
    // before draining would deadlock on a full channel.
    let spawn_fut = async move { spawn_az(&cfg, &argv, tx, cancel_rx).await; };
    let drain_fut = async {
        let mut exit_code: Option<i32> = None;
        let mut exit_duration_ms: u64 = 0;
        let mut stderr_first: Option<String> = None;
        let mut stderr_tail = String::new();
        let mut timed_out = false;
        while let Some(ev) = rx.recv().await {
            match ev {
                AzEvent::Stderr(line) => {
                    if stderr_first.is_none() { stderr_first = Some(line.clone()); }
                    if stderr_tail.len() < 2048 {
                        stderr_tail.push_str(&line);
                        stderr_tail.push('\n');
                    }
                }
                AzEvent::Exit { code, duration_ms } => { exit_code = Some(code); exit_duration_ms = duration_ms; break; }
                AzEvent::Timeout => { timed_out = true; break; }
                _ => {}
            }
        }
        (exit_code, exit_duration_ms, stderr_first, stderr_tail, timed_out)
    };
    let (_, (exit_code, exit_duration_ms, stderr_first, stderr_tail, timed_out)) =
        tokio::join!(spawn_fut, drain_fut);

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

    // Auth check must come before Missing classification — a non-zero exit
    // caused by an empty token cache would otherwise be mislabeled as
    // "resource does not exist".
    if exit_code != Some(0) && looks_like_not_logged_in(&stderr_tail) {
        cas_status(NodeStatus::Unverified)?;
        if let Some(app) = app {
            let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::AuthRequired {
                triggered_by: "verify".to_string(),
                logical_key: node_id.display(),
            }).unwrap());
        }
        return Err("not_logged_in".into());
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
    app: AppHandle,
    state: tauri::State<'_, SessionState>,
) -> Result<crate::model::NodeStatus, String> {
    do_verify_node(&logical_key, state.inner(), Some(&app)).await
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

    // Find the command tokens + its variable references.
    let (tokens_raw, var_refs): (Vec<String>, Vec<String>) = {
        let g = session.graph.lock().map_err(|e| e.to_string())?;
        let node = g.node(&node_id).ok_or_else(|| format!("node not found: {logical_key}"))?;
        let cmd_id = node.command_id.clone().ok_or_else(|| "node has no command".to_string())?;
        let cmd = g.commands().find(|c| c.id == cmd_id).ok_or_else(|| "command missing".to_string())?;
        (cmd.tokens.clone(), cmd.var_refs.clone())
    };

    // Resolve referenced variables before executing. Any still-Ghost with no
    // body blocks execute — the user must fill in a value in the detail pane.
    for var_name in &var_refs {
        let (body, cached) = {
            let g = session.graph.lock().map_err(|e| e.to_string())?;
            let v = g.variable(var_name).ok_or_else(|| format!("unknown variable ${var_name}"))?;
            (v.body.clone(), v.resolved.clone())
        };
        if cached.is_some() { continue; }
        use crate::model::VarBody;
        match body {
            VarBody::Unset => {
                return Err(format!("variable ${var_name} has no value set"));
            }
            VarBody::Literal { value } => {
                let mut g = session.graph.lock().map_err(|e| e.to_string())?;
                if let Some(v) = g.variable_mut(var_name) { v.resolved = Some(value); }
            }
            VarBody::Command { argv } => {
                let resolved = resolve_var_command(var_name, &argv, &app).await?;
                let mut g = session.graph.lock().map_err(|e| e.to_string())?;
                if let Some(v) = g.variable_mut(var_name) { v.resolved = Some(resolved); }
            }
        }
    }

    // Substitute `$NAME` in every token, using the now-cached resolved values.
    let argv: Vec<String> = {
        let g = session.graph.lock().map_err(|e| e.to_string())?;
        let resolve = |n: &str| -> Option<String> {
            g.variable(n).and_then(|v| v.resolved.clone())
        };
        tokens_raw.iter()
            .map(|t| crate::parser::varsyntax::substitute(t, &resolve))
            .collect()
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

    // Auth check: if the non-zero exit was caused by the user not being signed
    // in, don't mark the node Failed. Roll it back to Declared and emit an
    // auth-required event so the UI can prompt a login and offer a retry.
    if let Term::Exit { code, .. } = &term {
        if *code != 0 && crate::runner::looks_like_not_logged_in(&stderr_tail) {
            {
                let mut g = session.graph.lock().map_err(|e| e.to_string())?;
                if let Some(n) = g.node_mut(&node_id) {
                    n.status = crate::model::NodeStatus::Unverified;
                }
            }
            let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::AuthRequired {
                triggered_by: "execute".to_string(),
                logical_key: node_disp.clone(),
            }).unwrap());
            let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::Aborted {
                node: node_disp.clone(), reason: "not_logged_in".into(),
            }).unwrap());
            return Ok(());
        }
    }

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

/// Spawn `az login`. Streams each output line as a `login-log` run-event so
/// the existing LogPane can render it (the device-code URL lives in stderr).
/// Returns `Ok(())` on exit 0. A session-level mutex prevents overlapping
/// logins; a stored cancel sender lets the UI abort an in-flight sign-in.
#[tauri::command]
pub async fn az_login(app: AppHandle, state: tauri::State<'_, SessionState>) -> Result<(), String> {
    use crate::runner::{default_az_exe, spawn_az, AzConfig, AzEvent};
    use std::time::Duration;
    use tokio::sync::{mpsc, oneshot};

    let session = state.inner().clone();
    let _guard = session.login_lock.try_lock()
        .map_err(|_| "another login is already running".to_string())?;

    let (cancel_tx, cancel_rx) = oneshot::channel();
    *session.login_cancel.lock().map_err(|e| e.to_string())? = Some(cancel_tx);

    // 10-minute ceiling for device-code login. Az login itself can take a
    // while if the user walks away from the browser; this is an upper bound.
    let cfg = AzConfig { exe: default_az_exe().into(), timeout: Duration::from_secs(600) };
    let (tx, mut rx) = mpsc::channel::<AzEvent>(64);
    let argv = vec!["login".to_string()];

    let spawn_fut = async move { spawn_az(&cfg, &argv, tx, cancel_rx).await; };
    let drain_app = app.clone();
    let drain_fut = async move {
        let mut ok = false;
        let mut first_stderr: Option<String> = None;
        while let Some(ev) = rx.recv().await {
            match ev {
                AzEvent::Stdout(line) => {
                    let _ = drain_app.emit_all("run-event", serde_json::to_value(&RunEventWire::LoginLog {
                        line, is_err: false,
                    }).unwrap());
                }
                AzEvent::Stderr(line) => {
                    if first_stderr.is_none() { first_stderr = Some(line.clone()); }
                    let _ = drain_app.emit_all("run-event", serde_json::to_value(&RunEventWire::LoginLog {
                        line, is_err: true,
                    }).unwrap());
                }
                AzEvent::Exit { code, .. } => { ok = code == 0; break; }
                AzEvent::Timeout | AzEvent::Canceled => { ok = false; break; }
            }
        }
        (ok, first_stderr)
    };

    let (_, (ok, first_stderr)) = tokio::join!(spawn_fut, drain_fut);

    // Drop the cancel slot now that the process has finished.
    if let Ok(mut slot) = session.login_cancel.lock() { *slot = None; }

    let _ = app.emit_all("run-event", serde_json::to_value(&RunEventWire::LoginFinished { ok }).unwrap());
    if ok {
        Ok(())
    } else {
        Err(first_stderr.unwrap_or_else(|| "az login failed".into()))
    }
}

#[tauri::command]
pub fn az_login_cancel(state: tauri::State<SessionState>) -> Result<(), String> {
    let mut slot = state.login_cancel.lock().map_err(|e| e.to_string())?;
    if let Some(tx) = slot.take() { let _ = tx.send(()); }
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
    AuthRequired {
        triggered_by: String,
        logical_key: String,
    },
    LoginLog { line: String, is_err: bool },
    LoginFinished { ok: bool },
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
