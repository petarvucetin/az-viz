use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::model::Graph;
use crate::parser::ArgMap;

pub struct Session {
    pub graph: Mutex<Graph>,
    pub argmap: ArgMap,
    pub project_path: Mutex<Option<PathBuf>>,
}

pub type SessionState = Arc<Session>;

impl Session {
    pub fn new(argmap: ArgMap) -> Self {
        Self {
            graph: Mutex::new(Graph::new()),
            argmap,
            project_path: Mutex::new(None),
        }
    }
}
