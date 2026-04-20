use serde::{Deserialize, Serialize};

/// Where the variable came from.
/// - `Declared`: explicitly added by the user (`NAME=...`) or its body has
///   been filled in after being discovered.
/// - `Ghost`: discovered by `$NAME` reference in a consumer command but no
///   body has been set yet. Consumers referencing a ghost variable cannot
///   execute until the body is filled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarOrigin { Declared, Ghost }

/// Shape of the value source.
/// - `Command`: body is an argv to pass to `az`. Resolved by capturing
///   trimmed stdout. Starts with the literal token `az` after tokenization.
/// - `Literal`: body is a plain string; resolution is the string itself.
/// - `Unset`: ghost variable, no body filled in yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum VarBody {
    Command { argv: Vec<String> },
    Literal { value: String },
    Unset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub body: VarBody,
    pub origin: VarOrigin,
    /// Cached resolved value. `None` means not yet resolved (or invalidated).
    pub resolved: Option<String>,
}

impl Variable {
    pub fn ghost(name: impl Into<String>) -> Self {
        Self { name: name.into(), body: VarBody::Unset, origin: VarOrigin::Ghost, resolved: None }
    }
}
