#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod parser;
mod planner;
mod runner;
mod verify;
mod persist;
mod ipc;

use std::sync::Arc;
use ipc::{commands as ipc_cmd, Session};
use parser::ArgMap;

fn main() {
    let argmap_json = include_str!("../arg-map.json");
    let argmap = ArgMap::from_json(argmap_json).expect("bundled arg-map.json invalid");
    let session = Arc::new(Session::new(argmap));

    tauri::Builder::default()
        .manage(session)
        .invoke_handler(tauri::generate_handler![
            ipc_cmd::add_command,
            ipc_cmd::snapshot,
            ipc_cmd::dry_run,
            ipc_cmd::emit_script,
            ipc_cmd::open_project,
            ipc_cmd::save_project_as,
            ipc_cmd::run_live,
            ipc_cmd::remove_command,
            ipc_cmd::verify_node,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
