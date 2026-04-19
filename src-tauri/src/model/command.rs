use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    UnknownFlag(String),
    GhostReference(String),
    PropertyOverridden { key: String, previous: String, new: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning { pub kind: WarningKind, pub message: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub raw: String,
    pub tokens: Vec<String>,
    pub parsed_at: DateTime<Utc>,
    pub produces: NodeId,
    pub refs: Vec<NodeId>,
    #[serde(default)]
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};

    #[test]
    fn command_round_trips_through_json() {
        let rg = Scope::new("rg");
        let cmd = Command {
            id: "cmd-1".into(),
            raw: "az network vnet create --name v --resource-group rg".into(),
            tokens: vec!["az".into(), "network".into(), "vnet".into(), "create".into()],
            parsed_at: Utc::now(),
            produces: NodeId::of(NodeKind::Vnet, "v", &rg),
            refs: vec![],
            warnings: vec![],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cmd.id);
        assert_eq!(back.produces, cmd.produces);
    }
}
