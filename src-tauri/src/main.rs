#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod model;
mod parser;
mod planner;
mod runner;
mod verify;
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
