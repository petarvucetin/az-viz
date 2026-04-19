use std::collections::{HashMap, HashSet, VecDeque};
use crate::model::{Graph, NodeId};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PlanError {
    #[error("graph has a cycle involving: {0:?}")]
    Cycle(Vec<String>),
}

/// Kahn's algorithm. Deterministic: siblings are ordered by NodeId display string.
pub fn topo_order(graph: &Graph) -> Result<Vec<NodeId>, PlanError> {
    let ids: Vec<NodeId> = graph.nodes().map(|n| n.id.clone()).collect();
    let mut indegree: HashMap<NodeId, usize> = ids.iter().cloned().map(|i| (i, 0)).collect();
    for e in graph.edges() { *indegree.entry(e.to.clone()).or_insert(0) += 1; }

    let mut ready: Vec<NodeId> = indegree.iter().filter(|(_, d)| **d == 0).map(|(k, _)| k.clone()).collect();
    ready.sort_by_key(|id| id.display());
    let mut queue: VecDeque<NodeId> = VecDeque::from(ready);

    let mut out: Vec<NodeId> = vec![];
    let mut visited: HashSet<NodeId> = HashSet::new();

    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) { continue; }
        out.push(id.clone());
        let mut kids: Vec<NodeId> = graph.children(&id).cloned().collect();
        kids.sort_by_key(|i| i.display());
        for c in kids {
            let d = indegree.get_mut(&c).unwrap();
            *d -= 1;
            if *d == 0 { queue.push_back(c); }
        }
    }

    if out.len() < ids.len() {
        let leftover: Vec<String> = ids.iter().filter(|i| !visited.contains(*i)).map(|i| i.display()).collect();
        return Err(PlanError::Cycle(leftover));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, EdgeKind, Node, NodeKind};

    fn add(g: &mut Graph, kind: NodeKind, name: &str) -> NodeId {
        let n = Node::for_test(kind, name, "rg");
        let id = n.id.clone();
        g.add_node(n).unwrap();
        id
    }

    #[test]
    fn empty_graph_yields_empty_order() {
        let g = Graph::new();
        assert!(topo_order(&g).unwrap().is_empty());
    }

    #[test]
    fn parent_comes_before_child() {
        let mut g = Graph::new();
        let v = add(&mut g, NodeKind::Vnet, "v");
        let s = add(&mut g, NodeKind::Subnet, "s");
        g.add_edge(Edge { from: v.clone(), to: s.clone(), via: "--vnet-name".into(), kind: EdgeKind::Ref }).unwrap();
        let order = topo_order(&g).unwrap();
        assert_eq!(order, vec![v, s]);
    }

    proptest::proptest! {
        #[test]
        fn parent_always_before_child_on_random_dags(size in 3u32..20, edges in proptest::collection::vec((0u32..20, 0u32..20), 0..40)) {
            let mut g = Graph::new();
            let mut ids: Vec<NodeId> = vec![];
            for i in 0..size {
                ids.push(add(&mut g, NodeKind::Vnet, &format!("n{i}")));
            }
            for (a, b) in edges {
                let (a, b) = (a % size, b % size);
                if a < b {
                    let _ = g.add_edge(Edge {
                        from: ids[a as usize].clone(),
                        to: ids[b as usize].clone(),
                        via: "x".into(),
                        kind: EdgeKind::Ref,
                    });
                }
            }
            let order = topo_order(&g).unwrap();
            let pos: std::collections::HashMap<NodeId, usize> = order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
            for e in g.edges() {
                proptest::prop_assert!(pos[&e.from] < pos[&e.to]);
            }
        }
    }
}
