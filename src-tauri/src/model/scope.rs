use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    pub resource_group: String,
    pub subscription: Option<String>,
    pub location: Option<String>,
}

impl Scope {
    pub fn new(rg: impl Into<String>) -> Self {
        Self { resource_group: rg.into(), subscription: None, location: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_structural_equality_includes_location() {
        // Scope is a struct for carrying RG+sub+loc; identity semantics live in NodeId.
        let a = Scope { resource_group: "rg".into(), subscription: None, location: Some("westeurope".into()) };
        let b = Scope { resource_group: "rg".into(), subscription: None, location: None };
        assert_ne!(a, b, "Scope structural equality includes location");
    }
}
