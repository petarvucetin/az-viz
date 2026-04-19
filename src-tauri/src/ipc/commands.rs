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
