use az_plotter::ipc::state::Session;
use az_plotter::parser::ArgMap;
use std::sync::Arc;

fn session() -> Arc<Session> {
    let json = std::fs::read_to_string("arg-map.json").unwrap();
    Arc::new(Session::new(ArgMap::from_json(&json).unwrap()))
}

#[test]
fn add_commands_build_expected_graph() {
    let s = session();
    let mut g = s.graph.lock().unwrap();
    let p1 = az_plotter::parser::parse(
        "az network vnet create --name v --resource-group rg", &s.argmap, &g).unwrap();
    az_plotter::parser::commit(&mut g, p1).unwrap();
    let p2 = az_plotter::parser::parse(
        "az network vnet subnet create --name a --resource-group rg --vnet-name v", &s.argmap, &g).unwrap();
    az_plotter::parser::commit(&mut g, p2).unwrap();
    let p3 = az_plotter::parser::parse(
        "az network vnet subnet create --name b --resource-group rg --vnet-name v", &s.argmap, &g).unwrap();
    az_plotter::parser::commit(&mut g, p3).unwrap();

    assert_eq!(g.nodes().count(), 3);
    assert_eq!(g.edges().count(), 2);

    let plan = az_plotter::runner::dry_run(&g).unwrap();
    assert_eq!(plan.len(), 3);
    assert!(plan[0].argv.iter().any(|t| t == "vnet"));
    assert!(plan[0].argv.iter().any(|t| t == "create"));
}
