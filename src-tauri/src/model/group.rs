use serde::{Deserialize, Serialize};

/// A group of commands originating from the same `# <title>` comment block
/// in the Add textarea. Only groups with 2+ commands exist on the graph —
/// single-command sections don't become a Group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub title: String,
    /// Commands in declaration order (UI relies on this).
    pub command_ids: Vec<String>,
}
