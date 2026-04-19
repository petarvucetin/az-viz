fn main() {
    // `include_str!("../arg-map.json")` in main.rs embeds the argmap at compile time.
    // cargo's automatic tracking for include_str! is unreliable; state it explicitly.
    println!("cargo:rerun-if-changed=arg-map.json");
    tauri_build::build();
}
