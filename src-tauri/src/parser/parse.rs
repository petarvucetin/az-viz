use chrono::Utc;
use crate::model::{Command, Edge, EdgeKind, Graph, Node, NodeId, NodeKind, Scope, Variable, Warning, WarningKind};
use super::argmap::{ArgMap, ArgMapEntry};
use super::{tokenize, varsyntax};

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
    /// Ghost variables created because the command referenced `$NAME`
    /// for a variable not yet in the graph. Existing variables are left
    /// alone.
    pub new_variables: Vec<Variable>,
}

/// Top-level parse result: either a command, or a variable assignment.
/// Use `parse_line` as the entry point from the IPC layer; `parse` remains
/// the command-only path for existing tests.
#[derive(Debug)]
pub enum ParsedLine {
    Command(Parsed),
    Variable(Variable),
}

pub fn parse_line(line: &str, argmap: &ArgMap, graph: &Graph) -> Result<ParsedLine, ParseError> {
    if let Some((name, rhs)) = varsyntax::split_assignment(line) {
        return Ok(ParsedLine::Variable(varsyntax::variable_from_assignment(name, &rhs)));
    }
    parse(line, argmap, graph).map(ParsedLine::Command)
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
        "vnet-gateway" => NodeKind::VnetGateway,
        "local-gateway" => NodeKind::LocalGateway,
        "vpn-connection" => NodeKind::VpnConnection,
        "vnet-peering" => NodeKind::VnetPeering,
        "dns-resolver" => NodeKind::DnsResolver,
        "private-dns-zone" => NodeKind::PrivateDnsZone,
        "private-dns-link" => NodeKind::PrivateDnsLink,
        "rg" => NodeKind::ResourceGroup,
        _ => return None,
    })
}

fn short_alias(long_flag: &str) -> Option<&'static str> {
    match long_flag {
        "--resource-group" => Some("-g"),
        "--name"           => Some("-n"),
        "--location"       => Some("-l"),
        "--subscription"   => Some("-s"),
        _                  => None,
    }
}

/// Long-form aliases for Azure CLI flags that accept multiple canonical names.
/// `az` accepts both forms; the argmap declares one, we match either.
fn long_alias(long_flag: &str) -> Option<&'static str> {
    match long_flag {
        "--address-prefixes" => Some("--address-prefix"),
        _                    => None,
    }
}

fn flag_matches(token: &str, flag: &str) -> bool {
    token == flag
        || short_alias(flag).is_some_and(|s| token == s)
        || long_alias(flag).is_some_and(|a| token == a)
}

fn flag_eq_prefix<'a>(token: &'a str, flag: &str) -> Option<&'a str> {
    if let Some(v) = token.strip_prefix(&format!("{flag}=")) { return Some(v); }
    if let Some(a) = long_alias(flag) {
        if let Some(v) = token.strip_prefix(&format!("{a}=")) { return Some(v); }
    }
    None
}

fn flag_is_present(rest: &[String], flag: &str) -> bool {
    for t in rest {
        if flag_matches(t, flag) { return true; }
        if flag_eq_prefix(t, flag).is_some() { return true; }
    }
    false
}

fn extract_flag<'a>(rest: &'a [String], flag: &str) -> Option<&'a str> {
    let mut it = rest.iter();
    while let Some(t) = it.next() {
        if flag_matches(t, flag) {
            return it.next().map(|s| s.as_str());
        }
        if let Some(v) = flag_eq_prefix(t, flag) { return Some(v); }
    }
    None
}

/// Like `extract_flag` but collects all consecutive non-flag tokens after the flag.
/// For `--address-prefixes 10.0.0.0/26 10.0.1.0/26 --some-other-flag x`, returns
/// `vec!["10.0.0.0/26", "10.0.1.0/26"]`.
fn extract_flag_multi<'a>(rest: &'a [String], flag: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let t = &rest[i];
        if flag_matches(t, flag) {
            i += 1;
            while i < rest.len() && !rest[i].starts_with('-') {
                out.push(rest[i].as_str());
                i += 1;
            }
            return out;
        }
        if let Some(v) = flag_eq_prefix(t, flag) {
            out.push(v);
            return out;
        }
        i += 1;
    }
    out
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
    let mut produces_node = Node::declared(kind, name.clone(), scope.clone(), command_id.clone());
    let produces_id = produces_node.id.clone();

    // Populate declared props from argmap's `props` map.
    for (prop_name, flag) in &entry.props {
        let vals = extract_flag_multi(rest, flag);
        match vals.len() {
            0 => {
                // Flag might be a boolean present-without-value. If it's on the command line
                // at all, record `true`. Otherwise leave unset.
                if flag_is_present(rest, flag) {
                    produces_node.props.insert(prop_name.clone(), serde_json::Value::Bool(true));
                }
            }
            1 => {
                produces_node.props.insert(prop_name.clone(), serde_json::Value::String(vals[0].to_string()));
            }
            _ => {
                let arr = vals.iter().map(|s| serde_json::Value::String(s.to_string())).collect();
                produces_node.props.insert(prop_name.clone(), serde_json::Value::Array(arr));
            }
        }
    }

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

    // Scan every token for `$NAME` references. Collect unique names and
    // create Ghost variables for names not already in the graph.
    let full_tokens: Vec<String> = std::iter::once("az".to_string()).chain(tokens).collect();
    let mut seen_vars: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for t in &full_tokens {
        for name in varsyntax::scan_var_refs(t) { seen_vars.insert(name); }
    }
    let mut new_variables: Vec<Variable> = Vec::new();
    for name in &seen_vars {
        if graph.variable(name).is_none() && !new_variables.iter().any(|v| &v.name == name) {
            new_variables.push(Variable::ghost(name.clone()));
        }
    }
    let var_refs: Vec<String> = seen_vars.into_iter().collect();

    let command = Command {
        id: command_id,
        raw: line.to_string(),
        tokens: full_tokens,
        parsed_at: Utc::now(),
        produces: produces_id,
        refs: ref_ids,
        warnings,
        var_refs,
    };

    Ok(Parsed { command, new_nodes, new_edges, new_variables })
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
    fn short_flags_are_accepted() {
        let g = Graph::new();
        let m = load_argmap();
        // -g is the common short form of --resource-group; -n for --name
        let p = parse(
            "az network vnet create -g lakeflow-mssql-on-prem -n vnet-hub --address-prefixes 10.0.0.0/26 10.0.1.0/26",
            &m, &g,
        ).unwrap();
        assert_eq!(p.new_nodes.len(), 1);
        assert_eq!(p.new_nodes[0].kind, NodeKind::Vnet);
        assert_eq!(p.new_nodes[0].name, "vnet-hub");
        assert_eq!(p.new_nodes[0].id.resource_group, "lakeflow-mssql-on-prem");
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

    #[test]
    fn subnet_create_populates_cidr_prop() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefixes 10.0.0.0/27",
            &m, &g,
        ).unwrap();
        let subnet = p.new_nodes.iter().find(|n| n.kind == NodeKind::Subnet).unwrap();
        let cidr = subnet.props.get("cidr").expect("cidr prop missing");
        assert_eq!(cidr, &serde_json::json!("10.0.0.0/27"));
    }

    #[test]
    fn vnet_create_populates_multi_prefix_cidr() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network vnet create --name net-hub --resource-group rg --address-prefixes 10.0.0.0/26 10.0.1.0/26",
            &m, &g,
        ).unwrap();
        let vnet = p.new_nodes.iter().find(|n| n.kind == NodeKind::Vnet).unwrap();
        let cidr = vnet.props.get("cidr").expect("cidr prop missing");
        assert_eq!(cidr, &serde_json::json!(["10.0.0.0/26", "10.0.1.0/26"]));
    }

    #[test]
    fn subnet_create_accepts_singular_address_prefix_alias() {
        let g = Graph::new();
        let m = load_argmap();
        // --address-prefix (singular) is an Azure CLI alias of --address-prefixes
        let p = parse(
            "az network vnet subnet create --name s --resource-group rg --vnet-name v --address-prefix 10.0.0.0/27",
            &m, &g,
        ).unwrap();
        let subnet = p.new_nodes.iter().find(|n| n.kind == NodeKind::Subnet).unwrap();
        let cidr = subnet.props.get("cidr").expect("cidr prop missing under singular alias");
        assert_eq!(cidr, &serde_json::json!("10.0.0.0/27"));
    }

    #[test]
    fn nsg_create_without_cidr_has_no_cidr_prop() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network nsg create --name n --resource-group rg",
            &m, &g,
        ).unwrap();
        let nsg = &p.new_nodes[0];
        assert!(nsg.props.get("cidr").is_none());
    }

    #[test]
    fn quoted_dollar_var_ref_is_detected() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            r#"az network vpn-connection create -g rg -n cn-onprem --vnet-gateway1 vpngw-hub --local-gateway2 lng-onprem --shared-key "$PSK""#,
            &m, &g,
        ).unwrap();
        assert_eq!(p.command.var_refs, vec!["PSK".to_string()]);
        assert_eq!(p.new_variables.len(), 1);
        assert_eq!(p.new_variables[0].name, "PSK");
    }

    #[test]
    fn vnet_peering_captures_boolean_flags_as_true() {
        let g = Graph::new();
        let m = load_argmap();
        let p = parse(
            "az network vnet peering create -g rg --vnet-name v --name p --remote-vnet r --allow-vnet-access --allow-forwarded-traffic",
            &m, &g,
        ).unwrap();
        let peering = p.new_nodes.iter().find(|n| n.kind == NodeKind::VnetPeering).unwrap();
        assert_eq!(peering.props.get("allow-vnet-access"), Some(&serde_json::json!(true)));
        assert_eq!(peering.props.get("allow-forwarded-traffic"), Some(&serde_json::json!(true)));
        assert!(peering.props.get("allow-gateway-transit").is_none(), "absent flag should not be set");
    }
}
