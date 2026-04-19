// Dev utility: prints a skeleton arg-map entry by parsing `az network <cmd> --help` output.
// Not a full solution — emits a stub the maintainer then edits by hand before committing.

use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: regen-argmap <subcommand path> e.g. 'vnet create'");
        std::process::exit(2);
    }
    let path = args.join(" ");
    let out = Command::new("az").arg("network").args(path.split_whitespace()).arg("--help").output();
    let help = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => { eprintln!("failed to run az"); std::process::exit(1); }
    };

    println!("// skeleton for 'network {path}':");
    println!("\"network {path}\": {{");
    println!("  \"produces\": {{ \"kind\": \"REPLACE\", \"name_from\": \"--name\" }},");
    println!("  \"scope\":    {{ \"rg\": \"--resource-group\", \"location\": \"--location\" }},");
    println!("  \"refs\":     []");
    println!("}},");

    eprintln!("\n--- raw az help (first 40 lines) ---");
    for line in help.lines().take(40) { eprintln!("{line}"); }
}
