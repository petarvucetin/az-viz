use az_plotter::parser::ArgMap;
use std::fs;

#[test]
fn bundled_arg_map_loads_and_has_core_entries() {
    let text = fs::read_to_string("arg-map.json").expect("arg-map.json missing");
    let map = ArgMap::from_json(&text).expect("arg-map.json is valid JSON");
    for key in [
        "network vnet create",
        "network vnet subnet create",
        "network nsg create",
        "network nsg rule create",
        "network public-ip create",
        "network nic create",
    ] {
        assert!(map.lookup(key).is_some(), "missing entry: {key}");
    }
}
