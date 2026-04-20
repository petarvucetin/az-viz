use crate::model::{Graph, GraphError, Origin, NodeStatus};
use super::parse::{ParseError, Parsed};

pub fn commit(graph: &mut Graph, parsed: Parsed) -> Result<(), ParseError> {
    for n in parsed.new_nodes {
        if graph.node(&n.id).is_some() {
            // Node already exists. Merge props from the new node into the existing one.
            // If the new node is Declared and the existing was Ghost, promote it
            // (claim the command_id, reset status to Draft).
            let new_props = n.props.clone();
            let new_origin = n.origin.clone();
            let new_command_id = n.command_id.clone();
            let existing = graph.node_mut(&n.id).expect("just checked");
            for (k, v) in new_props { existing.props.insert(k, v); }
            if matches!(new_origin, Origin::Declared) && matches!(existing.origin, Origin::Ghost) {
                existing.origin = Origin::Declared;
                existing.command_id = new_command_id;
                existing.status = NodeStatus::Draft;
            }
        } else {
            graph.add_node(n).map_err(|e| match e {
                GraphError::Duplicate(s) => ParseError::Cycle(s),
                GraphError::NotFound(s) => ParseError::Cycle(s),
                GraphError::Cycle { from, to } => ParseError::Cycle(format!("{from} -> {to}")),
            })?;
        }
    }
    for e in parsed.new_edges {
        // Edges may now duplicate if two commands both create the same "ref" edge.
        // Silently ignore Duplicate edges.
        match graph.add_edge(e) {
            Ok(_) => {}
            Err(GraphError::Duplicate(_)) => {}
            Err(err) => {
                return Err(match err {
                    GraphError::Cycle { from, to } => ParseError::Cycle(format!("{from} -> {to}")),
                    GraphError::NotFound(s) => ParseError::Cycle(s),
                    GraphError::Duplicate(s) => ParseError::Cycle(s),
                });
            }
        }
    }
    graph.add_command(parsed.command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Graph, NodeKind};
    use crate::parser::{parse, ArgMap};

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/arg-map.json")).unwrap();
        ArgMap::from_json(&s).unwrap()
    }

    #[test]
    fn commit_inserts_nodes_edges_and_command() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p).unwrap();
        assert_eq!(g.nodes().count(), 1);
        assert_eq!(g.commands().count(), 1);
    }

    #[test]
    fn sequential_commits_draw_edges() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p1 = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p1).unwrap();
        let p2 = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p2).unwrap();
        assert_eq!(g.nodes().count(), 2);
        assert_eq!(g.edges().count(), 1);
        assert_eq!(g.nodes().filter(|n| n.kind == NodeKind::Vnet).count(), 1);
    }
}
