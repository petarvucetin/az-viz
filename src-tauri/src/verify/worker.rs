use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::model::{NodeId, NodeKind};
use crate::runner::az::{spawn_az, AzConfig, AzEvent};

#[derive(Debug, Clone)]
pub struct VerifyJob {
    pub node_id: NodeId,
    pub ref_key: u64,
}

#[derive(Debug, Clone)]
pub enum VerifyEvent {
    Started(NodeId),
    Result { node_id: NodeId, exists: bool },
    Stale(NodeId),
}

pub struct VerifierHandle {
    pub sender: mpsc::Sender<VerifyJob>,
    pub events: mpsc::Receiver<VerifyEvent>,
}

pub type RefKeyLookup = Arc<dyn Fn(&NodeId) -> Option<u64> + Send + Sync>;

pub fn hash_ref_key(id: &NodeId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

fn kind_to_az_subcommand(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Vnet => "vnet",
        NodeKind::Subnet => "vnet subnet",
        NodeKind::Nsg => "nsg",
        NodeKind::NsgRule => "nsg rule",
        NodeKind::PublicIp => "public-ip",
        NodeKind::Nic => "nic",
        NodeKind::Lb => "lb",
        NodeKind::RouteTable => "route-table",
        NodeKind::VnetGateway => "vnet-gateway",
        NodeKind::ResourceGroup => "group",
    }
}

pub fn spawn_worker(
    az_cfg: AzConfig,
    rate_per_minute: u32,
    lookup: RefKeyLookup,
) -> VerifierHandle {
    let (job_tx, mut job_rx) = mpsc::channel::<VerifyJob>(128);
    let (evt_tx, evt_rx) = mpsc::channel::<VerifyEvent>(128);

    let min_interval = if rate_per_minute == 0 {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(60_000 / rate_per_minute as u64)
    };

    tokio::spawn(async move {
        let mut last = Instant::now() - Duration::from_secs(60);
        while let Some(job) = job_rx.recv().await {
            let since = last.elapsed();
            if since < min_interval {
                tokio::time::sleep(min_interval - since).await;
            }
            last = Instant::now();

            if lookup(&job.node_id).map(|k| k != job.ref_key).unwrap_or(true) {
                let _ = evt_tx.send(VerifyEvent::Stale(job.node_id.clone())).await;
                continue;
            }

            let _ = evt_tx.send(VerifyEvent::Started(job.node_id.clone())).await;
            let sub = kind_to_az_subcommand(job.node_id.kind);
            let mut argv: Vec<String> = vec!["az".into()];
            argv.extend(sub.split_whitespace().map(String::from));
            argv.push("show".into());
            argv.push("--name".into()); argv.push(job.node_id.name.clone());
            argv.push("--resource-group".into()); argv.push(job.node_id.resource_group.clone());
            argv.push("--output".into()); argv.push("none".into());

            let (az_tx, mut az_rx) = mpsc::channel::<AzEvent>(32);
            let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
            let cfg_clone = az_cfg.clone();
            let argv_clone = argv.clone();
            tokio::spawn(async move {
                spawn_az(&cfg_clone, &argv_clone, az_tx, cancel_rx).await;
            });
            let mut exit: Option<i32> = None;
            while let Some(ev) = az_rx.recv().await {
                if let AzEvent::Exit { code, .. } = ev { exit = Some(code); break; }
                if matches!(ev, AzEvent::Timeout | AzEvent::Canceled) { exit = Some(-1); break; }
            }
            if lookup(&job.node_id).map(|k| k != job.ref_key).unwrap_or(true) {
                let _ = evt_tx.send(VerifyEvent::Stale(job.node_id.clone())).await;
                continue;
            }
            let exists = exit == Some(0);
            let _ = evt_tx.send(VerifyEvent::Result { node_id: job.node_id, exists }).await;
        }
    });

    VerifierHandle { sender: job_tx, events: evt_rx }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};
    use serial_test::serial;

    fn fake_az_path() -> String {
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe)
            .to_string_lossy().into_owned()
    }

    #[serial]
    #[tokio::test]
    async fn reports_exists_on_exit_zero() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "exit_code": 0 }]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let scope = Scope::new("rg");
        let id = NodeId::of(NodeKind::Vnet, "v", &scope);
        let expected = hash_ref_key(&id);
        let lookup: RefKeyLookup = Arc::new(move |_| Some(expected));
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let mut h = spawn_worker(cfg, 0, lookup);
        h.sender.send(VerifyJob { node_id: id.clone(), ref_key: expected }).await.unwrap();

        let mut result = None;
        while let Some(ev) = h.events.recv().await {
            if let VerifyEvent::Result { exists, .. } = ev { result = Some(exists); break; }
        }
        assert_eq!(result, Some(true));
    }

    #[serial]
    #[tokio::test]
    async fn stale_job_is_discarded() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "exit_code": 0 }]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let scope = Scope::new("rg");
        let id = NodeId::of(NodeKind::Vnet, "v", &scope);
        let lookup: RefKeyLookup = Arc::new(|_| Some(42));
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let mut h = spawn_worker(cfg, 0, lookup);
        h.sender.send(VerifyJob { node_id: id.clone(), ref_key: 7 }).await.unwrap();

        let mut saw_stale = false;
        while let Some(ev) = h.events.recv().await {
            if let VerifyEvent::Stale(_) = ev { saw_stale = true; break; }
            if let VerifyEvent::Result { .. } = ev { break; }
        }
        assert!(saw_stale);
    }
}
