use serde::{Deserialize, Serialize};
use super::scope::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    Vnet, Subnet, Nsg, NsgRule, PublicIp, Nic, Lb, RouteTable,
    #[serde(rename = "rg")]
    ResourceGroup,
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

    pub fn from_key(s: &str) -> Option<Self> {
        // Inverse of display(): "<kind>/<name>@rg:<rg>[/sub:<sub>]"
        let (kind_name, scope_part) = s.split_once('@')?;
        let (kind_str, name) = kind_name.split_once('/')?;
        if name.is_empty() { return None; }
        let kind = match kind_str {
            "vnet" => NodeKind::Vnet,
            "subnet" => NodeKind::Subnet,
            "nsg" => NodeKind::Nsg,
            "nsg-rule" => NodeKind::NsgRule,
            "public-ip" => NodeKind::PublicIp,
            "nic" => NodeKind::Nic,
            "lb" => NodeKind::Lb,
            "route-table" => NodeKind::RouteTable,
            "rg" => NodeKind::ResourceGroup,
            _ => return None,
        };
        let (rg_part, sub) = match scope_part.split_once("/sub:") {
            Some((rg, sub)) => (rg, Some(sub.to_string())),
            None => (scope_part, None),
        };
        let rg = rg_part.strip_prefix("rg:")?.to_string();
        Some(Self { kind, name: name.to_string(), resource_group: rg, subscription: sub })
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

    #[test]
    fn node_id_from_key_parses_without_subscription() {
        let id = NodeId::from_key("vnet/net-hub@rg:lakeflow").unwrap();
        assert_eq!(id.kind, NodeKind::Vnet);
        assert_eq!(id.name, "net-hub");
        assert_eq!(id.resource_group, "lakeflow");
        assert_eq!(id.subscription, None);
    }

    #[test]
    fn node_id_from_key_parses_with_subscription() {
        let id = NodeId::from_key("subnet/app@rg:rg1/sub:sub-abc").unwrap();
        assert_eq!(id.kind, NodeKind::Subnet);
        assert_eq!(id.name, "app");
        assert_eq!(id.resource_group, "rg1");
        assert_eq!(id.subscription, Some("sub-abc".into()));
    }

    #[test]
    fn node_id_from_key_rejects_bad_input() {
        assert!(NodeId::from_key("").is_none());
        assert!(NodeId::from_key("vnet/v").is_none());             // no @rg
        assert!(NodeId::from_key("bogus/v@rg:r").is_none());       // unknown kind
        assert!(NodeId::from_key("vnet/@rg:r").is_none());         // empty name
    }

    #[test]
    fn node_id_roundtrips_through_display_and_from_key() {
        let scope = Scope { resource_group: "my-rg".into(), subscription: Some("s1".into()), location: None };
        let id = NodeId::of(NodeKind::NsgRule, "rule-a", &scope);
        let back = NodeId::from_key(&id.display()).unwrap();
        assert_eq!(id, back);
    }
}
