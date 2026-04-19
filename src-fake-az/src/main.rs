use std::env;
use std::io::{self, Write};
use std::process::exit;
use std::time::Duration;

/// Reads AZ_FAKE_SCRIPT (JSON array of responses) and advances a counter stored in AZ_FAKE_STATE.
/// Each response: { "stdout": "...", "stderr": "...", "exit_code": 0, "sleep_ms": 0 }
fn main() {
    let script = env::var("AZ_FAKE_SCRIPT").unwrap_or_else(|_| "[]".to_string());
    let responses: Vec<serde_json::Value> = serde_json::from_str(&script).expect("AZ_FAKE_SCRIPT invalid");

    let state_path = env::var("AZ_FAKE_STATE").unwrap_or_else(|_| {
        let mut t = std::env::temp_dir();
        t.push("az-fake-state");
        t.to_string_lossy().to_string()
    });
    let idx: usize = std::fs::read_to_string(&state_path).ok()
        .and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    std::fs::write(&state_path, (idx + 1).to_string()).ok();

    let resp = responses.get(idx).cloned().unwrap_or_else(|| serde_json::json!({
        "stdout": "", "stderr": "", "exit_code": 0, "sleep_ms": 0
    }));

    if let Some(ms) = resp.get("sleep_ms").and_then(|v| v.as_u64()) {
        std::thread::sleep(Duration::from_millis(ms));
    }
    if let Some(s) = resp.get("stdout").and_then(|v| v.as_str()) {
        let _ = io::stdout().write_all(s.as_bytes());
    }
    if let Some(s) = resp.get("stderr").and_then(|v| v.as_str()) {
        let _ = io::stderr().write_all(s.as_bytes());
    }
    let code = resp.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    exit(code);
}
