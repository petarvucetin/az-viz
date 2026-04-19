use crate::model::{Graph, Node, NodeStatus, Origin};
use crate::runner::materialize::{materialize, MaterializeError, MaterializedCommand};

#[derive(Debug, thiserror::Error)]
pub enum ValidateError {
    #[error("unresolved required reference for {node}: {flag}")]
    UnresolvedRef { node: String, flag: String },
    #[error("reference to missing ghost for {0}")]
    MissingGhost(String),
    #[error(transparent)]
    Materialize(#[from] MaterializeError),
}

pub fn validate(graph: &Graph) -> Result<(), ValidateError> {
    for cmd in graph.commands() {
        let Some(produced) = graph.node(&cmd.produces) else { continue };
        if produced.origin != Origin::Declared { continue; }
        for ref_id in &cmd.refs {
            let Some(ref_node): Option<&Node> = graph.node(ref_id) else {
                return Err(ValidateError::UnresolvedRef {
                    node: cmd.produces.display(),
                    flag: ref_id.display(),
                });
            };
            if ref_node.origin == Origin::Ghost && ref_node.status == NodeStatus::Missing {
                return Err(ValidateError::MissingGhost(ref_id.display()));
            }
        }
    }
    Ok(())
}

pub fn dry_run(graph: &Graph) -> Result<Vec<MaterializedCommand>, ValidateError> {
    validate(graph)?;
    Ok(materialize(graph)?)
}
