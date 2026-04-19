use std::time::Duration;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AzEvent {
    Stdout(String),
    Stderr(String),
    Exit { code: i32, duration_ms: u64 },
    Timeout,
    Canceled,
}

#[derive(Clone)]
pub struct AzConfig {
    pub exe: String,
    pub timeout: Duration,
}

impl Default for AzConfig {
    fn default() -> Self { Self { exe: "az".into(), timeout: Duration::from_secs(300) } }
}

pub async fn spawn_az(
    cfg: &AzConfig,
    argv: &[String],
    tx: mpsc::Sender<AzEvent>,
    mut cancel: tokio::sync::oneshot::Receiver<()>,
) {
    let start = std::time::Instant::now();
    let args = if argv.first().map(|s| s.as_str()) == Some("az") { &argv[1..] } else { argv };
    let mut child = match Command::new(&cfg.exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AzEvent::Stderr(format!("spawn error: {e}"))).await;
            let _ = tx.send(AzEvent::Exit { code: -1, duration_ms: 0 }).await;
            return;
        }
    };
    let mut out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut err = BufReader::new(child.stderr.take().unwrap()).lines();

    let timeout = tokio::time::sleep(cfg.timeout);
    tokio::pin!(timeout);

    loop {
        select! {
            line = out.next_line() => {
                match line {
                    Ok(Some(l)) => { let _ = tx.send(AzEvent::Stdout(l)).await; }
                    _ => break,
                }
            }
            line = err.next_line() => {
                if let Ok(Some(l)) = line { let _ = tx.send(AzEvent::Stderr(l)).await; }
            }
            status = child.wait() => {
                let code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
                let _ = tx.send(AzEvent::Exit { code, duration_ms: start.elapsed().as_millis() as u64 }).await;
                return;
            }
            _ = &mut timeout => {
                let _ = child.kill().await;
                let _ = tx.send(AzEvent::Timeout).await;
                return;
            }
            _ = &mut cancel => {
                let _ = child.kill().await;
                let _ = tx.send(AzEvent::Canceled).await;
                return;
            }
        }
    }
    if let Ok(status) = child.wait().await {
        let code = status.code().unwrap_or(-1);
        let _ = tx.send(AzEvent::Exit { code, duration_ms: start.elapsed().as_millis() as u64 }).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn fake_az_path() -> String {
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe);
        p.to_string_lossy().into_owned()
    }

    #[serial]
    #[tokio::test]
    async fn runs_fake_az_and_reports_exit_zero() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "stdout": "hello\n", "exit_code": 0 }]"#);
        let state = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", state.path());
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let (tx, mut rx) = mpsc::channel(16);
        let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        spawn_az(&cfg, &["az".into(), "network".into(), "vnet".into(), "create".into()], tx, cancel_rx).await;
        let mut saw_stdout = false; let mut saw_exit = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                AzEvent::Stdout(s) if s == "hello" => saw_stdout = true,
                AzEvent::Exit { code: 0, .. } => saw_exit = true,
                _ => {}
            }
        }
        assert!(saw_stdout && saw_exit);
    }

    #[serial]
    #[tokio::test]
    async fn reports_nonzero_exit_on_failure() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "stderr": "boom\n", "exit_code": 2 }]"#);
        let state = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", state.path());
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let (tx, mut rx) = mpsc::channel(16);
        let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        spawn_az(&cfg, &["az".into()], tx, cancel_rx).await;
        let mut code = None;
        while let Ok(ev) = rx.try_recv() {
            if let AzEvent::Exit { code: c, .. } = ev { code = Some(c); }
        }
        assert_eq!(code, Some(2));
    }
}
