use chrono::Utc;
use crate::model::{Command, Edge, EdgeKind, Graph, Node, NodeId, NodeKind, Scope, Warning, WarningKind};
use super::argmap::{ArgMap, ArgMapEntry};
use super::tokenize;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error(transparent)]
    Tokenize(#[from] super::tokenize::TokenizeError),
    #[error("unknown subcommand path")]
    UnknownSubcommand,
    #[error("missing required flag: {0}")]
    MissingFlag(String),
    #[error("missing required resource-group (--resource-group)")]
    MissingResourceGroup,
    #[error("would create a graph cycle: {0}")]
    Cycle(String),
}

#[derive(Debug)]
pub struct Parsed {
    pub command: Command,
    pub new_nodes: Vec<Node>,
    pub new_edges: Vec<Edge>,
}

fn kind_from_str(s: &str) -> Option<NodeKind> {
    Some(match s {
        "vnet" => NodeKind::Vnet,
        "subnet" => NodeKind::Subnet,
        "nsg" => NodeKind::Nsg,
        "nsg-rule" => NodeKind::NsgRule,
        "public-ip" => NodeKind::PublicIp,
        "nic" => NodeKind::Nic,
        "lb" => NodeKind::Lb,
        "route-table" => NodeKind::RouteTable,
        "rg" => NodeKind::ResourceGroup,
        _ => return None,
    })
}

fn extract_flag<'a>(rest: &'a [String], flag: &str) -> Option<&'a str> {
    let mut it = rest.iter();
    while let Some(t) = it.next() {
        if t == flag { return it.next().map(|s| s.as_str()); }
        if let Some(v) = t.strip_prefix(&format!("{flag}=")) { return Some(v); }
    }
    None
}

pub fn parse(line: &str, argmap: &ArgMap, graph: &Graph) -> Result<Parsed, ParseError> {
    let tokens = tokenize::tokenize(line)?; // starts with "network"
    let (path, rest) = argmap.longest_match(&tokens).ok_or(ParseError::UnknownSubcommand)?;
    let entry: &ArgMapEntry = argmap.lookup(path).unwrap();

    // Scope: require --resource-group
    let rg_flag = entry.scope.rg.as_deref().unwrap_or("--resource-group");
    let rg = extract_flag(rest, rg_flag).ok_or(ParseError::MissingResourceGroup)?;
    let subscription = entry.scope.subscription.as_deref()
        .and_then(|f| extract_flag(rest, f))
        .map(|s| s.to_string());
    let location = entry.scope.location.as_deref()
        .and_then(|f| extract_flag(rest, f))
        .map(|s| s.to_string());
    let scope = Scope { resource_group: rg.to_string(), subscription, location };

    // Produced node
    let name = extract_flag(rest, &entry.produces.name_from)
        .ok_or_else(|| ParseError::MissingFlag(entry.produces.name_from.clone()))?
        .to_string();
    let kind = kind_from_str(&entry.produces.kind)
        .ok_or_else(|| ParseError::MissingFlag(format!("unknown kind: {}", entry.produces.kind)))?;
    let command_id = format!("cmd-{}", uuid::Uuid::new_v4());
    let produces_node = Node::declared(kind, name.clone(), scope.clone(), command_id.clone());
    let produces_id = produces_node.id.clone();

    // Refs
    let mut warnings: Vec<Warning> = vec![];
    let mut new_nodes: Vec<Node> = vec![];
    let mut new_edges: Vec<Edge> = vec![];
    let mut ref_ids: Vec<NodeId> = vec![];

    for spec in &entry.refs {
        let Some(val) = extract_flag(rest, &spec.via) else {
            if spec.required {
                return Err(ParseError::MissingFlag(spec.via.clone()));
            }
            continue;
        };
        let ref_kind = kind_from_str(&spec.kind)
            .ok_or_else(|| ParseError::MissingFlag(format!("unknown ref kind: {}", spec.kind)))?;
        let ref_id = NodeId::of(ref_kind, val.to_string(), &scope);
        if graph.node(&ref_id).is_none() && !new_nodes.iter().any(|n| n.id == ref_id) {
            let ghost = Node::ghost(ref_kind, val.to_string(), scope.clone());
            new_nodes.push(ghost);
            warnings.push(Warning {
                kind: WarningKind::GhostReference(ref_id.display()),
                message: format!("{} not found — added as ghost pending verification", ref_id.display()),
            });
        }
        new_edges.push(Edge { from: ref_id.clone(), to: produces_id.clone(), via: spec.via.clone(), kind: EdgeKind::Ref });
        ref_ids.push(ref_id);
    }

    new_nodes.push(produces_node);

    let command = Command {
        id: command_id,
        raw: line.to_string(),
        tokens: std::iter::once("az".to_string()).chain(tokens).collect(),
        parsed_at: Utc::now(),
        produces: produces_id,
        refs: ref_ids,
        warnings,
    };

    Ok(Parsed { command, new_nodes, new_edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/arg-map.json")).unwrap();
        ArgMap::from_json(&s).unwrap()
    }

    #[test]
    fn parses_vnet_create() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse("az network vnet create --name v --resource-group rg --address-prefix 10.0.0.0/16", &m, &g).unwrap();
        assert_eq!(p.new_nodes.len(), 1);
        assert_eq!(p.new_nodes[0].kind, NodeKind::Vnet);
        assert_eq!(p.new_nodes[0].name, "v");
        assert!(p.new_edges.is_empty());
    }

    #[test]
    fn subnet_with_missing_vnet_creates_ghost() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse("az network vnet subnet create --name s --resource-group rg --vnet-name ghosty", &m, &g).unwrap();
        assert_eq!(p.new_nodes.len(), 2);
        let ghost = p.new_nodes.iter().find(|n| n.kind == NodeKind::Vnet).unwrap();
        assert!(matches!(ghost.origin, crate::model::Origin::Ghost));
        assert_eq!(p.new_edges.len(), 1);
        assert_eq!(p.new_edges[0].via, "--vnet-name");
    }

    #[test]
    fn missing_required_flag_is_an_error() {
        let g = Graph::new();
        let m = load_argmap();
        let err = parse("az network vnet subnet create --name s --resource-group rg", &m, &g).unwrap_err();
        assert!(matches!(err, ParseError::MissingFlag(_)));
    }

    #[test]
    fn missing_resource_group_is_an_error() {
        let g = Graph::new();
        let m = load_argmap();
        let err = parse("az network vnet create --name v", &m, &g).unwrap_err();
        assert!(matches!(err, ParseError::MissingResourceGroup));
    }

    #[test]
    fn unknown_subcommand_is_an_error() {
        let g = Graph::new();
        let m = load_argmap();
        let err = parse("az network zzz create --name x --resource-group rg", &m, &g).unwrap_err();
        assert!(matches!(err, ParseError::UnknownSubcommand));
    }

    #[test]
    fn existing_declared_vnet_produces_edge_but_no_ghost() {
        use crate::model::Node;
        let mut g = Graph::new();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        g.add_node(v).unwrap();
        let m = load_argmap();
        let p = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        assert_eq!(p.new_nodes.iter().filter(|n| n.kind == NodeKind::Vnet).count(), 0);
        assert_eq!(p.new_edges.len(), 1);
    }
}
