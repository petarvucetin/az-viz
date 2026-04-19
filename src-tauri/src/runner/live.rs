use chrono::Utc;
use tokio::sync::{mpsc, oneshot};
use crate::model::{Graph, NodeId, NodeStatus};
use crate::runner::az::{spawn_az, AzConfig, AzEvent};
use crate::runner::dispatch::{validate, ValidateError};
use crate::runner::materialize::{materialize, MaterializedCommand};

#[derive(Debug, Clone)]
pub enum RunEvent {
    NodeStarted { node: NodeId, argv: Vec<String> },
    NodeLog { node: NodeId, line: String, is_err: bool },
    NodeFinished { node: NodeId, status: NodeStatus },
    Aborted { node: NodeId, reason: String },
    Done { succeeded: usize, failed: usize },
}

pub struct RunHandle {
    pub cancel: oneshot::Sender<()>,
    pub events: mpsc::Receiver<RunEvent>,
}

pub async fn live_run(
    graph: &Graph,
    az_cfg: AzConfig,
) -> Result<RunHandle, ValidateError> {
    validate(graph)?;
    let plan: Vec<MaterializedCommand> = materialize(graph)?;
    let (tx, rx) = mpsc::channel(128);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        for mc in plan {
            if cancel_rx.try_recv().is_ok() {
                let _ = tx.send(RunEvent::Aborted { node: mc.node_id.clone(), reason: "canceled".into() }).await;
                break;
            }
            let _ = tx.send(RunEvent::NodeStarted { node: mc.node_id.clone(), argv: mc.argv.clone() }).await;
            let (az_tx, mut az_rx) = mpsc::channel::<AzEvent>(64);
            let (node_cancel_tx, node_cancel_rx) = oneshot::channel();
            let tx_clone = tx.clone();
            let node_id = mc.node_id.clone();
            let cfg = az_cfg.clone();
            let argv = mc.argv.clone();
            let handle = tokio::spawn(async move {
                spawn_az(&cfg, &argv, az_tx, node_cancel_rx).await;
            });

            let mut exit_code: Option<i32> = None;
            let mut stderr_tail = String::new();
            let started = Utc::now();
            let mut node_cancel_tx = Some(node_cancel_tx);
            loop {
                tokio::select! {
                    ev = az_rx.recv() => {
                        match ev {
                            Some(AzEvent::Stdout(l)) => { let _ = tx_clone.send(RunEvent::NodeLog { node: node_id.clone(), line: l, is_err: false }).await; }
                            Some(AzEvent::Stderr(l)) => { stderr_tail = l.clone(); let _ = tx_clone.send(RunEvent::NodeLog { node: node_id.clone(), line: l, is_err: true }).await; }
                            Some(AzEvent::Exit { code, duration_ms: _ }) => { exit_code = Some(code); break; }
                            Some(AzEvent::Timeout) => { exit_code = Some(-1); stderr_tail = "timeout".into(); break; }
                            Some(AzEvent::Canceled) => { exit_code = Some(-1); stderr_tail = "canceled".into(); break; }
                            None => break,
                        }
                    }
                    _ = &mut cancel_rx => {
                        if let Some(tx) = node_cancel_tx.take() { let _ = tx.send(()); }
                    }
                }
            }
            let _ = handle.await;
            let dur = (Utc::now() - started).num_milliseconds().max(0) as u64;
            let status = match exit_code.unwrap_or(-1) {
                0 => NodeStatus::Succeeded { duration_ms: dur },
                c => NodeStatus::Failed { exit_code: c, stderr_tail: stderr_tail.clone(), duration_ms: dur },
            };
            let is_fail = matches!(status, NodeStatus::Failed { .. });
            let _ = tx.send(RunEvent::NodeFinished { node: node_id.clone(), status }).await;
            if is_fail {
                failed += 1;
                break;
            } else {
                succeeded += 1;
            }
        }
        let _ = tx.send(RunEvent::Done { succeeded, failed }).await;
    });

    Ok(RunHandle { cancel: cancel_tx, events: rx })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Graph;
    use crate::parser::{commit, parse, ArgMap};
    use serial_test::serial;

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/arg-map.json")).unwrap();
        ArgMap::from_json(&s).unwrap()
    }
    fn fake_az_path() -> String {
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe)
            .to_string_lossy().into_owned()
    }

    #[serial]
    #[tokio::test]
    async fn two_successful_commands_result_in_done_2_0() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p1 = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p1).unwrap();
        let p2 = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p2).unwrap();

        std::env::set_var("AZ_FAKE_SCRIPT", r#"[
            {"stdout":"ok1\n","exit_code":0},
            {"stdout":"ok2\n","exit_code":0}
        ]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let cfg = AzConfig { exe: fake_az_path(), timeout: std::time::Duration::from_secs(5) };
        let mut handle = live_run(&g, cfg).await.unwrap();

        let mut done = None;
        while let Some(ev) = handle.events.recv().await {
            if let RunEvent::Done { succeeded, failed } = ev { done = Some((succeeded, failed)); break; }
        }
        assert_eq!(done, Some((2, 0)));
    }
}
