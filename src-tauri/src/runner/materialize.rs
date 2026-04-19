use crate::model::{Command, Graph, NodeId, Origin};
use crate::planner::{topo_order, PlanError};

#[derive(Debug, Clone)]
pub struct MaterializedCommand {
    pub node_id: NodeId,
    pub command_id: String,
    pub argv: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MaterializeError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error("node has no originating command: {0}")]
    NoCommand(String),
}

pub fn materialize(graph: &Graph) -> Result<Vec<MaterializedCommand>, MaterializeError> {
    let order = topo_order(graph)?;
    let mut out = Vec::new();
    for id in order {
        let node = graph.node(&id).expect("topo returned unknown id");
        if node.origin == Origin::Ghost { continue; }
        let cmd_id = node.command_id.as_ref().ok_or_else(|| MaterializeError::NoCommand(id.display()))?;
        let cmd: &Command = graph.commands().find(|c| &c.id == cmd_id)
            .ok_or_else(|| MaterializeError::NoCommand(id.display()))?;
        out.push(MaterializedCommand {
            node_id: id,
            command_id: cmd.id.clone(),
            argv: cmd.tokens.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Graph;
    use crate::parser::{commit, parse, ArgMap};

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/arg-map.json")).unwrap();
        ArgMap::from_json(&s).unwrap()
    }

    #[test]
    fn materialize_yields_topological_order() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p).unwrap();
        // 'v' is ghost; subnet is declared. Ghost must be skipped, subnet must appear.
        let materialized = materialize(&g).unwrap();
        assert_eq!(materialized.len(), 1);
        assert_eq!(materialized[0].argv.first().map(|s| s.as_str()), Some("az"));
    }

    #[test]
    fn two_declared_nodes_come_out_in_dependency_order() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p1 = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p1).unwrap();
        let p2 = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p2).unwrap();
        let mat = materialize(&g).unwrap();
        assert_eq!(mat.len(), 2);
        assert!(mat[0].argv.iter().any(|t| t == "vnet"));
        assert!(mat[1].argv.iter().any(|t| t == "subnet"));
    }
}
