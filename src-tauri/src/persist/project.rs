use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::model::{Command, Graph};
use crate::parser::{commit, parse, ArgMap};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiState {
    #[serde(default)]
    pub layout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub commands: Vec<Command>,
    #[serde(default)]
    pub ui_state: UiState,
}

impl ProjectFile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn from_graph(graph: &Graph) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            commands: graph.commands().cloned().collect(),
            ui_state: UiState::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, s)
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s).map_err(std::io::Error::other)
    }

    pub fn to_graph(&self, argmap: &ArgMap) -> Result<Graph, crate::parser::ParseError> {
        let mut g = Graph::new();
        for c in &self.commands {
            let p = parse(&c.raw, argmap, &g)?;
            commit(&mut g, p)?;
        }
        Ok(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_argmap() -> ArgMap {
        ArgMap::from_json(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/arg-map.json")).unwrap()).unwrap()
    }

    #[test]
    fn round_trip_save_load_rebuild() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p1 = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p1).unwrap();
        let p2 = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p2).unwrap();

        let pf = ProjectFile::from_graph(&g);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        pf.save(tmp.path()).unwrap();

        let loaded = ProjectFile::load(tmp.path()).unwrap();
        assert_eq!(loaded.commands.len(), 2);
        let g2 = loaded.to_graph(&m).unwrap();
        assert_eq!(g2.nodes().count(), 2);
        assert_eq!(g2.edges().count(), 1);
    }
}
