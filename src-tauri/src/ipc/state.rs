use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::oneshot;
use crate::model::Graph;
use crate::parser::ArgMap;

pub struct Session {
    pub graph: Mutex<Graph>,
    pub argmap: ArgMap,
    pub project_path: Mutex<Option<PathBuf>>,
    /// Held for the duration of a per-node execute. Serializes them.
    pub execute_lock: AsyncMutex<()>,
    /// Held while an `az login` is running. Prevents overlapping logins.
    pub login_lock: AsyncMutex<()>,
    /// Cancel sender for the in-flight `az login`, if any.
    pub login_cancel: Mutex<Option<oneshot::Sender<()>>>,
}

pub type SessionState = Arc<Session>;

impl Session {
    pub fn new(argmap: ArgMap) -> Self {
        Self {
            graph: Mutex::new(Graph::new()),
            argmap,
            project_path: Mutex::new(None),
            execute_lock: AsyncMutex::new(()),
            login_lock: AsyncMutex::new(()),
            login_cancel: Mutex::new(None),
        }
    }
}
