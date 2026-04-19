use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::{NodeId, NodeKind, Scope};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin { Declared, Ghost }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeStatus {
    // declared
    Draft,
    Ready,
    Running { pid: u32, started_at: DateTime<Utc> },
    Succeeded { duration_ms: u64 },
    Failed { exit_code: i32, stderr_tail: String, duration_ms: u64 },
    Canceled,
    // ghost
    Unverified,
    Verifying,
    Exists,
    Missing,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self,
            NodeStatus::Succeeded { .. } | NodeStatus::Failed { .. } |
            NodeStatus::Canceled | NodeStatus::Exists | NodeStatus::Missing)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    pub scope: Scope,
    pub origin: Origin,
    pub status: NodeStatus,
    pub command_id: Option<String>,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

impl Node {
    pub fn declared(kind: NodeKind, name: impl Into<String>, scope: Scope, command_id: String) -> Self {
        let name = name.into();
        let id = NodeId::of(kind, name.clone(), &scope);
        Self {
            id, kind, name, scope,
            origin: Origin::Declared,
            status: NodeStatus::Draft,
            command_id: Some(command_id),
            props: BTreeMap::new(),
        }
    }

    pub fn ghost(kind: NodeKind, name: impl Into<String>, scope: Scope) -> Self {
        let name = name.into();
        let id = NodeId::of(kind, name.clone(), &scope);
        Self {
            id, kind, name, scope,
            origin: Origin::Ghost,
            status: NodeStatus::Unverified,
            command_id: None,
            props: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub fn for_test(kind: NodeKind, name: &str, rg: &str) -> Self {
        Self::declared(kind, name, Scope::new(rg), "cmd-test".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_node_starts_as_draft() {
        let n = Node::declared(NodeKind::Vnet, "v", Scope::new("rg"), "cmd-1".into());
        assert_eq!(n.status, NodeStatus::Draft);
        assert_eq!(n.origin, Origin::Declared);
    }

    #[test]
    fn ghost_node_starts_as_unverified() {
        let n = Node::ghost(NodeKind::Vnet, "v", Scope::new("rg"));
        assert_eq!(n.status, NodeStatus::Unverified);
        assert_eq!(n.origin, Origin::Ghost);
        assert!(n.command_id.is_none());
    }

    #[test]
    fn terminal_statuses_are_recognized() {
        assert!(NodeStatus::Succeeded { duration_ms: 1 }.is_terminal());
        assert!(NodeStatus::Failed { exit_code: 1, stderr_tail: "e".into(), duration_ms: 1 }.is_terminal());
        assert!(NodeStatus::Exists.is_terminal());
        assert!(!NodeStatus::Ready.is_terminal());
        assert!(!NodeStatus::Running { pid: 1, started_at: Utc::now() }.is_terminal());
    }
}
