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

#[test]
fn remove_command_autosaves_when_project_open() {
    let s = session();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let tmp_path = tmp.path().to_path_buf();
    drop(tmp); // let the file be created fresh by ProjectFile::save
    *s.project_path.lock().unwrap() = Some(tmp_path.clone());

    let cid = add(&s, "az network vnet create --name v --resource-group rg");
    az_plotter::ipc::commands::do_remove_command(&cid, &s).unwrap();

    // Autosave should have written the (now-empty) graph to tmp_path.
    let content = std::fs::read_to_string(&tmp_path).expect("autosave file should exist");
    assert!(!content.is_empty(), "autosave should have written something");
    // Clean up
    let _ = std::fs::remove_file(&tmp_path);
}
