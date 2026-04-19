use serde::{Deserialize, Serialize};
use super::scope::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NodeKind {
    Vnet, Subnet, Nsg, NsgRule, PublicIp, Nic, Lb, RouteTable, ResourceGroup,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Vnet => "vnet",
            NodeKind::Subnet => "subnet",
            NodeKind::Nsg => "nsg",
            NodeKind::NsgRule => "nsg-rule",
            NodeKind::PublicIp => "public-ip",
            NodeKind::Nic => "nic",
            NodeKind::Lb => "lb",
            NodeKind::RouteTable => "route-table",
            NodeKind::ResourceGroup => "rg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId {
    pub kind: NodeKind,
    pub name: String,
    pub resource_group: String,
    pub subscription: Option<String>,
}

impl NodeId {
    pub fn of(kind: NodeKind, name: impl Into<String>, scope: &Scope) -> Self {
        Self {
            kind,
            name: name.into(),
            resource_group: scope.resource_group.clone(),
            subscription: scope.subscription.clone(),
        }
    }

    /// Human-readable label for UI and log output.
    /// Not a canonical identifier — use the struct's derived `Hash`/`Eq` for map keys or persistence.
    pub fn display(&self) -> String {
        match &self.subscription {
            Some(sub) => format!("{}/{}@rg:{}/sub:{}", self.kind.as_str(), self.name, self.resource_group, sub),
            None => format!("{}/{}@rg:{}", self.kind.as_str(), self.name, self.resource_group),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EdgeKind { Ref, Scope }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display_omits_subscription_when_none() {
        let scope = Scope::new("my-rg");
        let id = NodeId::of(NodeKind::Vnet, "prod-hub", &scope);
        assert_eq!(id.display(), "vnet/prod-hub@rg:my-rg");
    }

    #[test]
    fn node_id_display_includes_subscription_when_set() {
        let scope = Scope {
            resource_group: "rg".into(),
            subscription: Some("sub1".into()),
            location: None,
        };
        let id = NodeId::of(NodeKind::Subnet, "app", &scope);
        assert_eq!(id.display(), "subnet/app@rg:rg/sub:sub1");
    }

    #[test]
    fn node_id_equality_ignores_location() {
        let s1 = Scope { resource_group: "rg".into(), subscription: None, location: Some("eastus".into()) };
        let s2 = Scope { resource_group: "rg".into(), subscription: None, location: None };
        assert_eq!(NodeId::of(NodeKind::Vnet, "v", &s1), NodeId::of(NodeKind::Vnet, "v", &s2));
    }
}
