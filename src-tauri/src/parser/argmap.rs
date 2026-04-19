use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Produces {
    pub kind: String,
    pub name_from: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScopeFlags {
    #[serde(default)] pub rg: Option<String>,
    #[serde(default)] pub subscription: Option<String>,
    #[serde(default)] pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefSpec {
    pub kind: String,
    pub via: String,
    #[serde(default)] pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgMapEntry {
    pub produces: Produces,
    #[serde(default)] pub scope: ScopeFlags,
    #[serde(default)] pub refs: Vec<RefSpec>,
    /// Map of prop-name → CLI flag. Parser reads these and populates `node.props`.
    /// Example: `{ "cidr": "--address-prefixes" }`.
    #[serde(default)] pub props: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArgMap(pub HashMap<String, ArgMapEntry>);

impl ArgMap {
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn merged(base: ArgMap, override_map: Option<ArgMap>) -> ArgMap {
        let mut out = base.0;
        if let Some(o) = override_map {
            out.extend(o.0);
        }
        ArgMap(out)
    }

    pub fn lookup(&self, path: &str) -> Option<&ArgMapEntry> {
        self.0.get(path)
    }

    /// Given tokens after "az", find the longest prefix that matches an arg-map entry.
    /// Returns (matched_path, remaining_tokens).
    pub fn longest_match<'a>(&self, tokens: &'a [String]) -> Option<(&str, &'a [String])> {
        for n in (1..=tokens.len()).rev() {
            let path = tokens[..n].join(" ");
            if let Some((k, _)) = self.0.get_key_value(&path) {
                return Some((k.as_str(), &tokens[n..]));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "network vnet create": {
        "produces": { "kind": "vnet", "name_from": "--name" },
        "scope":    { "rg": "--resource-group", "location": "--location" },
        "refs":     []
      },
      "network vnet subnet create": {
        "produces": { "kind": "subnet", "name_from": "--name" },
        "scope":    { "rg": "--resource-group" },
        "refs": [{ "kind": "vnet", "via": "--vnet-name", "required": true }]
      }
    }"#;

    #[test]
    fn parses_sample_json() {
        let m = ArgMap::from_json(SAMPLE).unwrap();
        assert!(m.lookup("network vnet create").is_some());
        assert_eq!(m.lookup("network vnet subnet create").unwrap().refs.len(), 1);
    }

    #[test]
    fn override_replaces_matching_key() {
        let base = ArgMap::from_json(SAMPLE).unwrap();
        let over = ArgMap::from_json(r#"{
          "network vnet create": {
            "produces": { "kind": "vnet", "name_from": "--renamed" },
            "scope": {},
            "refs": []
          }
        }"#).unwrap();
        let merged = ArgMap::merged(base, Some(over));
        assert_eq!(merged.lookup("network vnet create").unwrap().produces.name_from, "--renamed");
        assert!(merged.lookup("network vnet subnet create").is_some(), "override does not drop unrelated keys");
    }

    #[test]
    fn longest_match_prefers_longer_path() {
        let m = ArgMap::from_json(SAMPLE).unwrap();
        let tokens: Vec<String> = ["network","vnet","subnet","create","--name","s"]
            .iter().map(|s| s.to_string()).collect();
        let (path, rest) = m.longest_match(&tokens).unwrap();
        assert_eq!(path, "network vnet subnet create");
        assert_eq!(rest, &["--name", "s"]);
    }
}
