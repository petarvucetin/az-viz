use serde::{Deserialize, Serialize};
use super::{EdgeKind, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub via: String,
    pub kind: EdgeKind,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};

    #[test]
    fn edge_hashes_by_from_to_via() {
        use std::collections::HashSet;
        let rg = Scope::new("rg");
        let a = NodeId::of(NodeKind::Vnet, "v", &rg);
        let b = NodeId::of(NodeKind::Subnet, "s", &rg);
        let e1 = Edge { from: a.clone(), to: b.clone(), via: "--vnet-name".into(), kind: EdgeKind::Ref };
        let e2 = Edge { from: a, to: b, via: "--vnet-name".into(), kind: EdgeKind::Ref };
        let set: HashSet<_> = [e1, e2].into_iter().collect();
        assert_eq!(set.len(), 1);
    }
}
