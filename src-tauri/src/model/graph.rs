use std::collections::{BTreeMap, BTreeSet, HashMap};
use serde::{Deserialize, Serialize};
use super::{Command, Edge, Node, NodeId, NodeKind, Origin, Scope};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    nodes: HashMap<NodeId, Node>,
    edges: BTreeSet<Edge>,
    commands: BTreeMap<String, Command>,
    insertion_order: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GraphError {
    #[error("node already exists: {0}")]
    Duplicate(String),
    #[error("node not found: {0}")]
    NotFound(String),
    #[error("edge would create a cycle: {from} -> {to}")]
    Cycle { from: String, to: String },
}

impl Graph {
    pub fn new() -> Self { Self::default() }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> { self.nodes.values() }
    pub fn edges(&self) -> impl Iterator<Item = &Edge> { self.edges.iter() }
    pub fn commands(&self) -> impl Iterator<Item = &Command> {
        self.insertion_order.iter().filter_map(|id| self.commands.get(id))
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> { self.nodes.get(id) }
    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut Node> { self.nodes.get_mut(id) }

    pub fn find_by_identity(&self, kind: NodeKind, name: &str, scope: &Scope) -> Option<&Node> {
        let candidate = NodeId::of(kind, name, scope);
        self.nodes.get(&candidate)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId, GraphError> {
        if self.nodes.contains_key(&node.id) {
            return Err(GraphError::Duplicate(node.id.display()));
        }
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        Ok(id)
    }

    pub fn promote_ghost(&mut self, id: &NodeId, command_id: String) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(id).ok_or_else(|| GraphError::NotFound(id.display()))?;
        node.origin = Origin::Declared;
        node.status = super::NodeStatus::Draft;
        node.command_id = Some(command_id);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(GraphError::NotFound(edge.from.display()));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(GraphError::NotFound(edge.to.display()));
        }
        if self.would_create_cycle(&edge.from, &edge.to) {
            return Err(GraphError::Cycle {
                from: edge.from.display(),
                to: edge.to.display(),
            });
        }
        self.edges.insert(edge);
        Ok(())
    }

    pub fn add_command(&mut self, cmd: Command) {
        let id = cmd.id.clone();
        if !self.commands.contains_key(&id) {
            self.insertion_order.push(id.clone());
        }
        self.commands.insert(id, cmd);
    }

    pub fn parents<'a>(&'a self, id: &'a NodeId) -> impl Iterator<Item = &'a NodeId> + 'a {
        self.edges.iter().filter(move |e| e.to == *id).map(|e| &e.from)
    }

    pub fn children<'a>(&'a self, id: &'a NodeId) -> impl Iterator<Item = &'a NodeId> + 'a {
        self.edges.iter().filter(move |e| e.from == *id).map(|e| &e.to)
    }

    fn would_create_cycle(&self, from: &NodeId, to: &NodeId) -> bool {
        // Adding from -> to creates a cycle iff `from` is already reachable from `to`.
        let mut stack = vec![to.clone()];
        let mut seen = BTreeSet::new();
        while let Some(cur) = stack.pop() {
            if &cur == from { return true; }
            if !seen.insert(cur.clone()) { continue; }
            for child in self.children(&cur) { stack.push(child.clone()); }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EdgeKind, Node, NodeKind};

    fn mk_graph() -> Graph { Graph::new() }

    #[test]
    fn add_and_lookup_node() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let id = g.add_node(v).unwrap();
        assert!(g.node(&id).is_some());
    }

    #[test]
    fn duplicate_node_is_rejected() {
        let mut g = mk_graph();
        g.add_node(Node::for_test(NodeKind::Vnet, "v", "rg")).unwrap();
        let err = g.add_node(Node::for_test(NodeKind::Vnet, "v", "rg")).unwrap_err();
        assert!(matches!(err, GraphError::Duplicate(_)));
    }

    #[test]
    fn edge_with_missing_endpoint_is_rejected() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let s = Node::for_test(NodeKind::Subnet, "s", "rg");
        let vid = v.id.clone();
        let sid = s.id.clone();
        g.add_node(v).unwrap();
        let err = g.add_edge(Edge { from: vid, to: sid, via: "--vnet-name".into(), kind: EdgeKind::Ref }).unwrap_err();
        assert!(matches!(err, GraphError::NotFound(_)));
    }

    #[test]
    fn cycle_is_rejected() {
        let mut g = mk_graph();
        let a = Node::for_test(NodeKind::Vnet, "a", "rg");
        let b = Node::for_test(NodeKind::Subnet, "b", "rg");
        let aid = a.id.clone();
        let bid = b.id.clone();
        g.add_node(a).unwrap();
        g.add_node(b).unwrap();
        g.add_edge(Edge { from: aid.clone(), to: bid.clone(), via: "x".into(), kind: EdgeKind::Ref }).unwrap();
        let err = g.add_edge(Edge { from: bid, to: aid, via: "y".into(), kind: EdgeKind::Ref }).unwrap_err();
        assert!(matches!(err, GraphError::Cycle { .. }));
    }
}
