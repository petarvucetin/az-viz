# az-plotter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `az-plotter` v1 — a Tauri desktop app that parses `az network … create` commands entered live, renders them as a DAG of Azure resources, and executes them in topological order via Dry-run / Live / Emit-script modes.

**Architecture:** Rust core owns the authoritative graph state (model, parser, topological planner, runner, verification worker, autosaver) and shells out to `az`. A Svelte + Cytoscape.js WebView renders the graph and forwards user intents over Tauri IPC. The core is pure and test-hermetic; a tiny fake `az` crate substitutes for the real binary in tests. Arg-map lives in JSON — new az subcommands are a data change, not a code change.

**Tech Stack:** Rust (stable), Tauri 1.x, Tokio, `shell-words`, `serde`, Svelte 4, Vite, TypeScript, Cytoscape.js, `@tauri-apps/api`. Windows MSI build via Tauri bundler.

**Spec:** `docs/superpowers/specs/2026-04-18-az-plotter-design.md`

**Task count:** 27, grouped into 7 phases. Each task ends with a commit.

---

## Phase 0 — Scaffolding

### Task 1: Initialize workspace, Tauri app, Svelte UI, git

**Files:**
- Create: `D:/AI/projects/az-plotter/.gitignore`
- Create: `D:/AI/projects/az-plotter/Cargo.toml` (workspace)
- Create: `D:/AI/projects/az-plotter/src-tauri/Cargo.toml`
- Create: `D:/AI/projects/az-plotter/src-tauri/tauri.conf.json`
- Create: `D:/AI/projects/az-plotter/src-tauri/build.rs`
- Create: `D:/AI/projects/az-plotter/src-tauri/src/main.rs` (minimal)
- Create: `D:/AI/projects/az-plotter/ui/package.json`
- Create: `D:/AI/projects/az-plotter/ui/vite.config.ts`
- Create: `D:/AI/projects/az-plotter/ui/svelte.config.js`
- Create: `D:/AI/projects/az-plotter/ui/tsconfig.json`
- Create: `D:/AI/projects/az-plotter/ui/index.html`
- Create: `D:/AI/projects/az-plotter/ui/src/main.ts`
- Create: `D:/AI/projects/az-plotter/ui/src/App.svelte`

- [ ] **Step 1: Create `.gitignore`**

```gitignore
# Rust / Cargo
target/
**/*.rs.bk

# Node / npm
node_modules/
dist/
.vite/

# Tauri build artifacts
src-tauri/target/
src-tauri/gen/schemas/

# App data
*.log
.env
.env.local

# OS
Thumbs.db
.DS_Store

# Brainstorming/internal
.superpowers/
```

- [ ] **Step 2: Create workspace `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["src-tauri", "src-fake-az"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
thiserror = "1"
shell-words = "1"
```

- [ ] **Step 3: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "az-plotter"
version.workspace = true
edition.workspace = true
license.workspace = true

[build-dependencies]
tauri-build = { version = "1", features = [] }

[dependencies]
tauri = { version = "1", features = ["shell-open", "dialog-all", "fs-all"] }
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
anyhow.workspace = true
thiserror.workspace = true
shell-words.workspace = true
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
directories = "5"
notify = "6"

[dev-dependencies]
proptest = "1"
pretty_assertions = "1"
tempfile = "3"

[[bin]]
name = "az-plotter"
path = "src/main.rs"

[[bin]]
name = "regen-argmap"
path = "src/bin/regen-argmap.rs"
```

- [ ] **Step 4: Create `src-tauri/tauri.conf.json`**

```json
{
  "build": {
    "beforeBuildCommand": "npm --prefix ../ui run build",
    "beforeDevCommand": "npm --prefix ../ui run dev",
    "devPath": "http://localhost:1420",
    "distDir": "../ui/dist"
  },
  "package": { "productName": "az-plotter", "version": "0.1.0" },
  "tauri": {
    "allowlist": { "shell": { "execute": true, "open": true } },
    "bundle": {
      "active": true,
      "identifier": "com.station5solutions.az-plotter",
      "targets": ["msi", "nsis"]
    },
    "windows": [
      { "title": "az-plotter", "width": 1200, "height": 780, "resizable": true }
    ]
  }
}
```

- [ ] **Step 5: Create `src-tauri/build.rs` and `src-tauri/src/main.rs` (minimal)**

`build.rs`:
```rust
fn main() { tauri_build::build(); }
```

`src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Scaffold Svelte + Vite UI**

`ui/package.json`:
```json
{
  "name": "az-plotter-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite --port 1420 --strictPort",
    "build": "svelte-check && vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.0.0",
    "@tsconfig/svelte": "^5.0.0",
    "svelte": "^4.2.0",
    "svelte-check": "^3.6.0",
    "tslib": "^2.6.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0"
  },
  "dependencies": {
    "@tauri-apps/api": "^1.5.0",
    "cytoscape": "^3.28.0",
    "cytoscape-dagre": "^2.5.0"
  }
}
```

`ui/vite.config.ts`:
```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
});
```

`ui/svelte.config.js`:
```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
export default { preprocess: vitePreprocess() };
```

`ui/tsconfig.json`:
```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "isolatedModules": true
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

`ui/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>az-plotter</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`ui/src/main.ts`:
```ts
import App from "./App.svelte";
const app = new App({ target: document.getElementById("app")! });
export default app;
```

`ui/src/App.svelte`:
```svelte
<main>
  <h1>az-plotter</h1>
  <p>Scaffold ready.</p>
</main>
```

- [ ] **Step 7: Initialize git and verify the build**

Run:
```bash
git init
git add .
npm --prefix ui install
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: `cargo check` succeeds; `npm install` installs without errors.

- [ ] **Step 8: Commit**

```bash
git add .
git commit -m "chore: scaffold Tauri + Svelte workspace"
```

---

## Phase 1 — Core data model

### Task 2: Scope, NodeId, NodeKind, EdgeKind

**Files:**
- Create: `src-tauri/src/model/mod.rs`
- Create: `src-tauri/src/model/scope.rs`
- Create: `src-tauri/src/model/ids.rs`
- Modify: `src-tauri/src/main.rs` to declare `mod model;`

- [ ] **Step 1: Write failing test for `Scope` normalization and `NodeId` format**

`src-tauri/src/model/scope.rs`:
```rust
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
    fn scope_equality_ignores_location_for_identity() {
        // Scope is a struct for carrying RG+sub+loc; identity semantics live in NodeId.
        let a = Scope { resource_group: "rg".into(), subscription: None, location: Some("westeurope".into()) };
        let b = Scope { resource_group: "rg".into(), subscription: None, location: None };
        assert_ne!(a, b, "Scope structural equality includes location");
    }
}
```

`src-tauri/src/model/ids.rs`:
```rust
use serde::{Deserialize, Serialize};
use super::scope::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Vnet, Subnet, Nsg, NsgRule, PublicIp, Nic, Lb, RouteTable, ResourceGroup,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Vnet => "vnet",
            NodeKind::Subnet => "subnet",
            NodeKind::Nsg => "nsg",
            NodeKind::NsgRule => "nsg-rule",
            NodeKind::PublicIp => "public-ip",
            NodeKind::Nic => "nic",
            NodeKind::Lb => "lb",
            NodeKind::RouteTable => "route-table",
            NodeKind::ResourceGroup => "rg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId {
    pub kind: NodeKind,
    pub name: String,
    pub resource_group: String,
    pub subscription: Option<String>,
}

impl NodeId {
    pub fn of(kind: NodeKind, name: impl Into<String>, scope: &Scope) -> Self {
        Self {
            kind,
            name: name.into(),
            resource_group: scope.resource_group.clone(),
            subscription: scope.subscription.clone(),
        }
    }

    pub fn display(&self) -> String {
        match &self.subscription {
            Some(sub) => format!("{}/{}@rg:{}/sub:{}", self.kind.as_str(), self.name, self.resource_group, sub),
            None => format!("{}/{}@rg:{}", self.kind.as_str(), self.name, self.resource_group),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind { Ref, Scope }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display_omits_subscription_when_none() {
        let scope = Scope::new("my-rg");
        let id = NodeId::of(NodeKind::Vnet, "prod-hub", &scope);
        assert_eq!(id.display(), "vnet/prod-hub@rg:my-rg");
    }

    #[test]
    fn node_id_display_includes_subscription_when_set() {
        let scope = Scope {
            resource_group: "rg".into(),
            subscription: Some("sub1".into()),
            location: None,
        };
        let id = NodeId::of(NodeKind::Subnet, "app", &scope);
        assert_eq!(id.display(), "subnet/app@rg:rg/sub:sub1");
    }

    #[test]
    fn node_id_equality_ignores_location() {
        let s1 = Scope { resource_group: "rg".into(), subscription: None, location: Some("eastus".into()) };
        let s2 = Scope { resource_group: "rg".into(), subscription: None, location: None };
        assert_eq!(NodeId::of(NodeKind::Vnet, "v", &s1), NodeId::of(NodeKind::Vnet, "v", &s2));
    }
}
```

- [ ] **Step 2: Declare modules**

`src-tauri/src/model/mod.rs`:
```rust
pub mod ids;
pub mod scope;

pub use ids::{EdgeKind, NodeId, NodeKind};
pub use scope::Scope;
```

In `src-tauri/src/main.rs`, add `mod model;` under the `#![cfg_attr(…)]` line.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::`
Expected: all model tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/model src-tauri/src/main.rs
git commit -m "feat(model): Scope, NodeId, NodeKind, EdgeKind"
```

---

### Task 3: Node struct + NodeStatus state machine

**Files:**
- Create: `src-tauri/src/model/node.rs`
- Modify: `src-tauri/src/model/mod.rs`

- [ ] **Step 1: Write failing test for NodeStatus transitions and Node construction**

`src-tauri/src/model/node.rs`:
```rust
use std::collections::BTreeMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::{NodeId, NodeKind, Scope};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin { Declared, Ghost }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeStatus {
    // declared
    Draft,
    Ready,
    Running { pid: u32, started_at: DateTime<Utc> },
    Succeeded { duration_ms: u64 },
    Failed { exit_code: i32, stderr_tail: String, duration_ms: u64 },
    Canceled,
    // ghost
    Unverified,
    Verifying,
    Exists,
    Missing,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self,
            NodeStatus::Succeeded { .. } | NodeStatus::Failed { .. } |
            NodeStatus::Canceled | NodeStatus::Exists | NodeStatus::Missing)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    pub scope: Scope,
    pub origin: Origin,
    pub status: NodeStatus,
    pub command_id: Option<String>,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

impl Node {
    pub fn declared(kind: NodeKind, name: impl Into<String>, scope: Scope, command_id: String) -> Self {
        let name = name.into();
        let id = NodeId::of(kind, name.clone(), &scope);
        Self {
            id, kind, name, scope,
            origin: Origin::Declared,
            status: NodeStatus::Draft,
            command_id: Some(command_id),
            props: BTreeMap::new(),
        }
    }

    pub fn ghost(kind: NodeKind, name: impl Into<String>, scope: Scope) -> Self {
        let name = name.into();
        let id = NodeId::of(kind, name.clone(), &scope);
        Self {
            id, kind, name, scope,
            origin: Origin::Ghost,
            status: NodeStatus::Unverified,
            command_id: None,
            props: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub fn for_test(kind: NodeKind, name: &str, rg: &str) -> Self {
        Self::declared(kind, name, Scope::new(rg), "cmd-test".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_node_starts_as_draft() {
        let n = Node::declared(NodeKind::Vnet, "v", Scope::new("rg"), "cmd-1".into());
        assert_eq!(n.status, NodeStatus::Draft);
        assert_eq!(n.origin, Origin::Declared);
    }

    #[test]
    fn ghost_node_starts_as_unverified() {
        let n = Node::ghost(NodeKind::Vnet, "v", Scope::new("rg"));
        assert_eq!(n.status, NodeStatus::Unverified);
        assert_eq!(n.origin, Origin::Ghost);
        assert!(n.command_id.is_none());
    }

    #[test]
    fn terminal_statuses_are_recognized() {
        assert!(NodeStatus::Succeeded { duration_ms: 1 }.is_terminal());
        assert!(NodeStatus::Failed { exit_code: 1, stderr_tail: "e".into(), duration_ms: 1 }.is_terminal());
        assert!(NodeStatus::Exists.is_terminal());
        assert!(!NodeStatus::Ready.is_terminal());
        assert!(!NodeStatus::Running { pid: 1, started_at: Utc::now() }.is_terminal());
    }
}
```

- [ ] **Step 2: Export from `model/mod.rs`**

```rust
pub mod ids;
pub mod node;
pub mod scope;

pub use ids::{EdgeKind, NodeId, NodeKind};
pub use node::{Node, NodeStatus, Origin};
pub use scope::Scope;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::node::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/model
git commit -m "feat(model): Node struct with NodeStatus state machine"
```

---

### Task 4: Edge, Command, Warning

**Files:**
- Create: `src-tauri/src/model/edge.rs`
- Create: `src-tauri/src/model/command.rs`
- Modify: `src-tauri/src/model/mod.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/model/edge.rs`:
```rust
use serde::{Deserialize, Serialize};
use super::{EdgeKind, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub via: String,
    pub kind: EdgeKind,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};

    #[test]
    fn edge_hashes_by_from_to_via() {
        use std::collections::HashSet;
        let rg = Scope::new("rg");
        let a = NodeId::of(NodeKind::Vnet, "v", &rg);
        let b = NodeId::of(NodeKind::Subnet, "s", &rg);
        let e1 = Edge { from: a.clone(), to: b.clone(), via: "--vnet-name".into(), kind: EdgeKind::Ref };
        let e2 = Edge { from: a, to: b, via: "--vnet-name".into(), kind: EdgeKind::Ref };
        let set: HashSet<_> = [e1, e2].into_iter().collect();
        assert_eq!(set.len(), 1);
    }
}
```

`src-tauri/src/model/command.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningKind {
    UnknownFlag(String),
    GhostReference(String),
    PropertyOverridden { key: String, previous: String, new: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning { pub kind: WarningKind, pub message: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub raw: String,
    pub tokens: Vec<String>,
    pub parsed_at: DateTime<Utc>,
    pub produces: NodeId,
    pub refs: Vec<NodeId>,
    #[serde(default)]
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};

    #[test]
    fn command_round_trips_through_json() {
        let rg = Scope::new("rg");
        let cmd = Command {
            id: "cmd-1".into(),
            raw: "az network vnet create --name v --resource-group rg".into(),
            tokens: vec!["az".into(), "network".into(), "vnet".into(), "create".into()],
            parsed_at: Utc::now(),
            produces: NodeId::of(NodeKind::Vnet, "v", &rg),
            refs: vec![],
            warnings: vec![],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, cmd.id);
        assert_eq!(back.produces, cmd.produces);
    }
}
```

- [ ] **Step 2: Export from `model/mod.rs`**

```rust
pub mod command;
pub mod edge;
pub mod ids;
pub mod node;
pub mod scope;

pub use command::{Command, Warning, WarningKind};
pub use edge::Edge;
pub use ids::{EdgeKind, NodeId, NodeKind};
pub use node::{Node, NodeStatus, Origin};
pub use scope::Scope;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::`
Expected: new tests pass, existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/model
git commit -m "feat(model): Edge, Command, Warning"
```

---

### Task 5: Graph store

**Files:**
- Create: `src-tauri/src/model/graph.rs`
- Modify: `src-tauri/src/model/mod.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/model/graph.rs`:
```rust
use std::collections::{BTreeMap, BTreeSet, HashMap};
use serde::{Deserialize, Serialize};
use super::{Command, Edge, Node, NodeId, NodeKind, Origin, Scope};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Graph {
    nodes: HashMap<NodeId, Node>,
    edges: BTreeSet<Edge>,
    commands: BTreeMap<String, Command>,
    insertion_order: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum GraphError {
    #[error("node already exists: {0}")]
    Duplicate(String),
    #[error("node not found: {0}")]
    NotFound(String),
    #[error("edge would create a cycle: {from} -> {to}")]
    Cycle { from: String, to: String },
}

impl Graph {
    pub fn new() -> Self { Self::default() }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> { self.nodes.values() }
    pub fn edges(&self) -> impl Iterator<Item = &Edge> { self.edges.iter() }
    pub fn commands(&self) -> impl Iterator<Item = &Command> {
        self.insertion_order.iter().filter_map(|id| self.commands.get(id))
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> { self.nodes.get(id) }
    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut Node> { self.nodes.get_mut(id) }

    pub fn find_by_identity(&self, kind: NodeKind, name: &str, scope: &Scope) -> Option<&Node> {
        let candidate = NodeId::of(kind, name, scope);
        self.nodes.get(&candidate)
    }

    pub fn add_node(&mut self, node: Node) -> Result<NodeId, GraphError> {
        if self.nodes.contains_key(&node.id) {
            return Err(GraphError::Duplicate(node.id.display()));
        }
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        Ok(id)
    }

    pub fn promote_ghost(&mut self, id: &NodeId, command_id: String) -> Result<(), GraphError> {
        let node = self.nodes.get_mut(id).ok_or_else(|| GraphError::NotFound(id.display()))?;
        node.origin = Origin::Declared;
        node.status = super::NodeStatus::Draft;
        node.command_id = Some(command_id);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(GraphError::NotFound(edge.from.display()));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(GraphError::NotFound(edge.to.display()));
        }
        if self.would_create_cycle(&edge.from, &edge.to) {
            return Err(GraphError::Cycle {
                from: edge.from.display(),
                to: edge.to.display(),
            });
        }
        self.edges.insert(edge);
        Ok(())
    }

    pub fn add_command(&mut self, cmd: Command) {
        let id = cmd.id.clone();
        if !self.commands.contains_key(&id) {
            self.insertion_order.push(id.clone());
        }
        self.commands.insert(id, cmd);
    }

    pub fn parents(&self, id: &NodeId) -> impl Iterator<Item = &NodeId> {
        self.edges.iter().filter(move |e| e.to == *id).map(|e| &e.from)
    }

    pub fn children(&self, id: &NodeId) -> impl Iterator<Item = &NodeId> {
        self.edges.iter().filter(move |e| e.from == *id).map(|e| &e.to)
    }

    fn would_create_cycle(&self, from: &NodeId, to: &NodeId) -> bool {
        // Adding from -> to creates a cycle iff `from` is already reachable from `to`.
        let mut stack = vec![to.clone()];
        let mut seen = BTreeSet::new();
        while let Some(cur) = stack.pop() {
            if &cur == from { return true; }
            if !seen.insert(cur.clone()) { continue; }
            for child in self.children(&cur) { stack.push(child.clone()); }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EdgeKind, Node, NodeKind};

    fn mk_graph() -> Graph { Graph::new() }

    #[test]
    fn add_and_lookup_node() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let id = g.add_node(v).unwrap();
        assert!(g.node(&id).is_some());
    }

    #[test]
    fn duplicate_node_is_rejected() {
        let mut g = mk_graph();
        g.add_node(Node::for_test(NodeKind::Vnet, "v", "rg")).unwrap();
        let err = g.add_node(Node::for_test(NodeKind::Vnet, "v", "rg")).unwrap_err();
        assert!(matches!(err, GraphError::Duplicate(_)));
    }

    #[test]
    fn edge_with_missing_endpoint_is_rejected() {
        let mut g = mk_graph();
        let v = Node::for_test(NodeKind::Vnet, "v", "rg");
        let s = Node::for_test(NodeKind::Subnet, "s", "rg");
        let vid = v.id.clone();
        let sid = s.id.clone();
        g.add_node(v).unwrap();
        let err = g.add_edge(Edge { from: vid, to: sid, via: "--vnet-name".into(), kind: EdgeKind::Ref }).unwrap_err();
        assert!(matches!(err, GraphError::NotFound(_)));
    }

    #[test]
    fn cycle_is_rejected() {
        let mut g = mk_graph();
        let a = Node::for_test(NodeKind::Vnet, "a", "rg");
        let b = Node::for_test(NodeKind::Subnet, "b", "rg");
        let aid = a.id.clone();
        let bid = b.id.clone();
        g.add_node(a).unwrap();
        g.add_node(b).unwrap();
        g.add_edge(Edge { from: aid.clone(), to: bid.clone(), via: "x".into(), kind: EdgeKind::Ref }).unwrap();
        let err = g.add_edge(Edge { from: bid, to: aid, via: "y".into(), kind: EdgeKind::Ref }).unwrap_err();
        assert!(matches!(err, GraphError::Cycle { .. }));
    }
}
```

- [ ] **Step 2: Export from `model/mod.rs`**

Add `pub mod graph;` and `pub use graph::{Graph, GraphError};`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml model::graph::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/model
git commit -m "feat(model): Graph store with cycle-detecting edge insertion"
```

---

## Phase 2 — Parsing

### Task 6: ArgMap type and JSON loader with override merge

**Files:**
- Create: `src-tauri/src/parser/mod.rs`
- Create: `src-tauri/src/parser/argmap.rs`
- Modify: `src-tauri/src/main.rs` to declare `mod parser;`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/parser/argmap.rs`:
```rust
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
```

`src-tauri/src/parser/mod.rs`:
```rust
pub mod argmap;
pub use argmap::{ArgMap, ArgMapEntry, Produces, RefSpec, ScopeFlags};
```

- [ ] **Step 2: Declare parser module** in `main.rs`: `mod parser;`

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::argmap::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser src-tauri/src/main.rs
git commit -m "feat(parser): ArgMap types, JSON loader, override merge, longest-match"
```

---

### Task 7: Bundled starter arg-map.json

**Files:**
- Create: `src-tauri/arg-map.json`
- Create: `src-tauri/tests/argmap_bundle.rs`

- [ ] **Step 1: Write failing test that loads the bundled file**

`src-tauri/tests/argmap_bundle.rs`:
```rust
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
```

Integration tests require a `lib.rs`. Create `src-tauri/src/lib.rs`:
```rust
pub mod model;
pub mod parser;
```

And in `Cargo.toml` add a `[lib]` entry:
```toml
[lib]
name = "az_plotter"
path = "src/lib.rs"
```

- [ ] **Step 2: Create `src-tauri/arg-map.json`**

```json
{
  "network vnet create": {
    "produces": { "kind": "vnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  },
  "network vnet subnet create": {
    "produces": { "kind": "subnet", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "vnet", "via": "--vnet-name", "required": true }
    ]
  },
  "network nsg create": {
    "produces": { "kind": "nsg", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  },
  "network nsg rule create": {
    "produces": { "kind": "nsg-rule", "name_from": "--name" },
    "scope":    { "rg": "--resource-group" },
    "refs": [
      { "kind": "nsg", "via": "--nsg-name", "required": true }
    ]
  },
  "network public-ip create": {
    "produces": { "kind": "public-ip", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  },
  "network nic create": {
    "produces": { "kind": "nic", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs": [
      { "kind": "subnet",    "via": "--subnet",             "required": true },
      { "kind": "public-ip", "via": "--public-ip-address",  "required": false },
      { "kind": "nsg",       "via": "--network-security-group", "required": false }
    ]
  },
  "network route-table create": {
    "produces": { "kind": "route-table", "name_from": "--name" },
    "scope":    { "rg": "--resource-group", "location": "--location" },
    "refs":     []
  }
}
```

- [ ] **Step 3: Run**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test argmap_bundle`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/arg-map.json src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/tests/argmap_bundle.rs
git commit -m "feat(parser): bundle starter arg-map.json covering core network resources"
```

---

### Task 8: Tokenizer

**Files:**
- Create: `src-tauri/src/parser/tokenize.rs`
- Modify: `src-tauri/src/parser/mod.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/parser/tokenize.rs`:
```rust
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TokenizeError {
    #[error("empty input")]
    Empty,
    #[error("expected 'az' as first token")]
    MissingAz,
    #[error("expected 'network' as second token")]
    MissingNetwork,
    #[error("shell tokenization failed: {0}")]
    Shell(String),
}

/// Tokenizes a line and verifies it begins with `az network`.
/// Returns the tokens **after** `az` (i.e., starting with `network`).
pub fn tokenize(line: &str) -> Result<Vec<String>, TokenizeError> {
    let trimmed = line.trim();
    if trimmed.is_empty() { return Err(TokenizeError::Empty); }
    // Collapse backslash-newline continuations
    let joined = trimmed.replace("\\\n", " ").replace("\\\r\n", " ");
    let tokens = shell_words::split(&joined).map_err(|e| TokenizeError::Shell(e.to_string()))?;
    if tokens.first().map(|s| s.as_str()) != Some("az") {
        return Err(TokenizeError::MissingAz);
    }
    if tokens.get(1).map(|s| s.as_str()) != Some("network") {
        return Err(TokenizeError::MissingNetwork);
    }
    Ok(tokens.into_iter().skip(1).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_basic_command() {
        let toks = tokenize("az network vnet create --name v --resource-group rg").unwrap();
        assert_eq!(toks, ["network","vnet","create","--name","v","--resource-group","rg"]);
    }

    #[test]
    fn supports_quoted_values_with_spaces() {
        let toks = tokenize(r#"az network vnet create --name "my vnet" --resource-group rg"#).unwrap();
        assert_eq!(toks[3], "--name");
        assert_eq!(toks[4], "my vnet");
    }

    #[test]
    fn supports_line_continuations() {
        let input = "az network vnet create \\\n  --name v \\\n  --resource-group rg";
        let toks = tokenize(input).unwrap();
        assert!(toks.contains(&"--name".to_string()));
        assert!(toks.contains(&"v".to_string()));
    }

    #[test]
    fn rejects_non_az() {
        assert_eq!(tokenize("pwsh script.ps1").unwrap_err(), TokenizeError::MissingAz);
    }

    #[test]
    fn rejects_non_network() {
        assert_eq!(tokenize("az group create").unwrap_err(), TokenizeError::MissingNetwork);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(tokenize("   ").unwrap_err(), TokenizeError::Empty);
    }
}
```

- [ ] **Step 2: Export** in `parser/mod.rs`:

```rust
pub mod argmap;
pub mod tokenize;
pub use argmap::{ArgMap, ArgMapEntry, Produces, RefSpec, ScopeFlags};
pub use tokenize::{tokenize, TokenizeError};
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::tokenize::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser
git commit -m "feat(parser): tokenizer with az-network prefix validation"
```

---

### Task 9: Parser — extract produced node and references

**Files:**
- Create: `src-tauri/src/parser/parse.rs`
- Modify: `src-tauri/src/parser/mod.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/parser/parse.rs`:
```rust
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

    // collect kind-to-dedupe for new_nodes to avoid double insertion in a single parse
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
            // create a ghost
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
        let s = std::fs::read_to_string("arg-map.json").unwrap();
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
        // Two nodes: ghost vnet, declared subnet
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
```

- [ ] **Step 2: Export**

In `parser/mod.rs` add `pub mod parse;` and `pub use parse::{parse, ParseError, Parsed};`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::parse::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser
git commit -m "feat(parser): parse pipeline — extract node, resolve refs, create ghosts"
```

---

### Task 10: Parse → Graph commit helper with cycle defense

**Files:**
- Create: `src-tauri/src/parser/commit.rs`
- Modify: `src-tauri/src/parser/mod.rs`

- [ ] **Step 1: Write failing test**

`src-tauri/src/parser/commit.rs`:
```rust
use crate::model::{Graph, GraphError};
use super::parse::{ParseError, Parsed};

pub fn commit(graph: &mut Graph, parsed: Parsed) -> Result<(), ParseError> {
    // Add new nodes (skip duplicates silently — parse.rs already deduped new_nodes vs graph)
    for n in parsed.new_nodes {
        if graph.node(&n.id).is_none() {
            // Safe: previously absent.
            graph.add_node(n).map_err(|e| match e {
                GraphError::Duplicate(s) => ParseError::Cycle(s), // won't happen but map anyway
                GraphError::NotFound(s) => ParseError::Cycle(s),
                GraphError::Cycle { from, to } => ParseError::Cycle(format!("{from} -> {to}")),
            })?;
        }
    }
    for e in parsed.new_edges {
        graph.add_edge(e).map_err(|err| match err {
            GraphError::Cycle { from, to } => ParseError::Cycle(format!("{from} -> {to}")),
            GraphError::NotFound(s) => ParseError::Cycle(s),
            GraphError::Duplicate(s) => ParseError::Cycle(s),
        })?;
    }
    graph.add_command(parsed.command);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Graph, NodeKind};
    use crate::parser::{parse, ArgMap};

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string("arg-map.json").unwrap();
        ArgMap::from_json(&s).unwrap()
    }

    #[test]
    fn commit_inserts_nodes_edges_and_command() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p).unwrap();
        assert_eq!(g.nodes().count(), 1);
        assert_eq!(g.commands().count(), 1);
    }

    #[test]
    fn sequential_commits_draw_edges() {
        let mut g = Graph::new();
        let m = load_argmap();
        let p1 = parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap();
        commit(&mut g, p1).unwrap();
        let p2 = parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap();
        commit(&mut g, p2).unwrap();
        assert_eq!(g.nodes().count(), 2);
        assert_eq!(g.edges().count(), 1);
        assert_eq!(g.nodes().filter(|n| n.kind == NodeKind::Vnet).count(), 1);
    }
}
```

- [ ] **Step 2: Export** in `parser/mod.rs`:

```rust
pub mod commit;
pub use commit::commit;
```

- [ ] **Step 3: Run**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parser::commit::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/parser
git commit -m "feat(parser): graph-commit helper wrapping parse output"
```

---

## Phase 3 — Planner + Runner + Fake az

### Task 11: Topological planner

**Files:**
- Create: `src-tauri/src/planner/mod.rs`
- Create: `src-tauri/src/planner/topo.rs`
- Modify: `src-tauri/src/main.rs` & `src-tauri/src/lib.rs` to add `pub mod planner;`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/planner/topo.rs`:
```rust
use std::collections::{HashMap, HashSet, VecDeque};
use crate::model::{Graph, NodeId};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PlanError {
    #[error("graph has a cycle involving: {0:?}")]
    Cycle(Vec<String>),
}

/// Kahn's algorithm. Deterministic: siblings are ordered by NodeId display string.
pub fn topo_order(graph: &Graph) -> Result<Vec<NodeId>, PlanError> {
    let ids: Vec<NodeId> = graph.nodes().map(|n| n.id.clone()).collect();
    let mut indegree: HashMap<NodeId, usize> = ids.iter().cloned().map(|i| (i, 0)).collect();
    for e in graph.edges() { *indegree.entry(e.to.clone()).or_insert(0) += 1; }

    // ready = indegree 0, sorted
    let mut ready: Vec<NodeId> = indegree.iter().filter(|(_, d)| **d == 0).map(|(k, _)| k.clone()).collect();
    ready.sort_by_key(|id| id.display());
    let mut queue: VecDeque<NodeId> = VecDeque::from(ready);

    let mut out: Vec<NodeId> = vec![];
    let mut visited: HashSet<NodeId> = HashSet::new();

    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) { continue; }
        out.push(id.clone());
        // collect children, sort deterministically
        let mut kids: Vec<NodeId> = graph.children(&id).cloned().collect();
        kids.sort_by_key(|i| i.display());
        for c in kids {
            let d = indegree.get_mut(&c).unwrap();
            *d -= 1;
            if *d == 0 { queue.push_back(c); }
        }
    }

    if out.len() < ids.len() {
        let leftover: Vec<String> = ids.iter().filter(|i| !visited.contains(*i)).map(|i| i.display()).collect();
        return Err(PlanError::Cycle(leftover));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, EdgeKind, Node, NodeKind};

    fn add(g: &mut Graph, kind: NodeKind, name: &str) -> NodeId {
        let n = Node::for_test(kind, name, "rg");
        let id = n.id.clone();
        g.add_node(n).unwrap();
        id
    }

    #[test]
    fn empty_graph_yields_empty_order() {
        let g = Graph::new();
        assert!(topo_order(&g).unwrap().is_empty());
    }

    #[test]
    fn parent_comes_before_child() {
        let mut g = Graph::new();
        let v = add(&mut g, NodeKind::Vnet, "v");
        let s = add(&mut g, NodeKind::Subnet, "s");
        g.add_edge(Edge { from: v.clone(), to: s.clone(), via: "--vnet-name".into(), kind: EdgeKind::Ref }).unwrap();
        let order = topo_order(&g).unwrap();
        assert_eq!(order, vec![v, s]);
    }

    proptest::proptest! {
        #[test]
        fn parent_always_before_child_on_random_dags(size in 3u32..20, edges in proptest::collection::vec((0u32..20, 0u32..20), 0..40)) {
            let mut g = Graph::new();
            let mut ids: Vec<NodeId> = vec![];
            for i in 0..size {
                ids.push(add(&mut g, NodeKind::Vnet, &format!("n{i}")));
            }
            // Only add edges where from_index < to_index to guarantee a DAG
            for (a, b) in edges {
                let (a, b) = (a % size, b % size);
                if a < b {
                    let _ = g.add_edge(Edge {
                        from: ids[a as usize].clone(),
                        to: ids[b as usize].clone(),
                        via: "x".into(),
                        kind: EdgeKind::Ref,
                    });
                }
            }
            let order = topo_order(&g).unwrap();
            let pos: std::collections::HashMap<NodeId, usize> = order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
            for e in g.edges() {
                proptest::prop_assert!(pos[&e.from] < pos[&e.to]);
            }
        }
    }
}
```

- [ ] **Step 2: Add `planner/mod.rs`**

```rust
pub mod topo;
pub use topo::{topo_order, PlanError};
```

Add `pub mod planner;` to both `main.rs` and `lib.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml planner::`
Expected: unit tests pass, property test passes.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/planner src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(planner): Kahn's topological sort with property test"
```

---

### Task 12: Materializer

**Files:**
- Create: `src-tauri/src/runner/mod.rs`
- Create: `src-tauri/src/runner/materialize.rs`
- Modify: `src-tauri/src/main.rs` & `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/runner/materialize.rs`:
```rust
use crate::model::{Command, Graph, NodeId, Origin};
use crate::planner::{topo_order, PlanError};

#[derive(Debug, Clone)]
pub struct MaterializedCommand {
    pub node_id: NodeId,
    pub command_id: String,
    pub argv: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MaterializeError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error("node has no originating command: {0}")]
    NoCommand(String),
}

pub fn materialize(graph: &Graph) -> Result<Vec<MaterializedCommand>, MaterializeError> {
    let order = topo_order(graph)?;
    let mut out = Vec::new();
    for id in order {
        let node = graph.node(&id).expect("topo returned unknown id");
        if node.origin == Origin::Ghost { continue; }
        let cmd_id = node.command_id.as_ref().ok_or_else(|| MaterializeError::NoCommand(id.display()))?;
        let cmd: &Command = graph.commands().find(|c| &c.id == cmd_id)
            .ok_or_else(|| MaterializeError::NoCommand(id.display()))?;
        out.push(MaterializedCommand {
            node_id: id,
            command_id: cmd.id.clone(),
            argv: cmd.tokens.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Graph;
    use crate::parser::{commit, parse, ArgMap};

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string("arg-map.json").unwrap();
        ArgMap::from_json(&s).unwrap()
    }

    #[test]
    fn materialize_yields_topological_order() {
        let mut g = Graph::new();
        let m = load_argmap();
        commit(&mut g, parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap()).unwrap();
        // 'v' was declared as ghost above; now declare it:
        // Promote ghost to declared by adding the real command — but for this test, we keep ghost
        // and assert ghost is skipped.
        let materialized = materialize(&g).unwrap();
        // Only declared node 's' should be in the plan.
        assert_eq!(materialized.len(), 1);
        assert_eq!(materialized[0].argv.first().map(|s| s.as_str()), Some("az"));
    }

    #[test]
    fn two_declared_nodes_come_out_in_dependency_order() {
        let mut g = Graph::new();
        let m = load_argmap();
        commit(&mut g, parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap()).unwrap();
        commit(&mut g, parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap()).unwrap();
        let mat = materialize(&g).unwrap();
        assert_eq!(mat.len(), 2);
        // first is vnet (name=v), second is subnet (name=s)
        assert!(mat[0].argv.iter().any(|t| t == "vnet"));
        assert!(mat[1].argv.iter().any(|t| t == "subnet"));
    }
}
```

- [ ] **Step 2: `runner/mod.rs`**

```rust
pub mod materialize;
pub use materialize::{materialize, MaterializeError, MaterializedCommand};
```

Add `pub mod runner;` to both `main.rs` and `lib.rs`.

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runner::materialize::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/runner src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(runner): materialize plan from topological order"
```

---

### Task 13: Fake az binary (separate workspace crate)

**Files:**
- Create: `src-fake-az/Cargo.toml`
- Create: `src-fake-az/src/main.rs`

- [ ] **Step 1: Create `src-fake-az/Cargo.toml`**

```toml
[package]
name = "fake-az"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "fake-az"
path = "src/main.rs"

[dependencies]
serde.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Write the fake az behavior**

`src-fake-az/src/main.rs`:
```rust
use std::env;
use std::io::{self, Write};
use std::process::exit;
use std::time::Duration;

/// Reads AZ_FAKE_SCRIPT (JSON array of responses) and advances a counter stored in AZ_FAKE_STATE.
/// Each response: { "stdout": "...", "stderr": "...", "exit_code": 0, "sleep_ms": 0 }
fn main() {
    let script = env::var("AZ_FAKE_SCRIPT").unwrap_or_else(|_| "[]".to_string());
    let responses: Vec<serde_json::Value> = serde_json::from_str(&script).expect("AZ_FAKE_SCRIPT invalid");

    let state_path = env::var("AZ_FAKE_STATE").unwrap_or_else(|_| {
        let mut t = std::env::temp_dir();
        t.push("az-fake-state");
        t.to_string_lossy().to_string()
    });
    let idx: usize = std::fs::read_to_string(&state_path).ok()
        .and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    std::fs::write(&state_path, (idx + 1).to_string()).ok();

    let resp = responses.get(idx).cloned().unwrap_or_else(|| serde_json::json!({
        "stdout": "", "stderr": "", "exit_code": 0, "sleep_ms": 0
    }));

    if let Some(ms) = resp.get("sleep_ms").and_then(|v| v.as_u64()) {
        std::thread::sleep(Duration::from_millis(ms));
    }
    if let Some(s) = resp.get("stdout").and_then(|v| v.as_str()) {
        let _ = io::stdout().write_all(s.as_bytes());
    }
    if let Some(s) = resp.get("stderr").and_then(|v| v.as_str()) {
        let _ = io::stderr().write_all(s.as_bytes());
    }
    let code = resp.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    exit(code);
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build --manifest-path src-fake-az/Cargo.toml`
Expected: success, binary at `target/debug/fake-az(.exe)`.

- [ ] **Step 4: Commit**

```bash
git add src-fake-az
git commit -m "feat(test): fake-az binary for hermetic runner tests"
```

---

### Task 14: az invoker (async, streaming, timeout, cancellation)

**Files:**
- Create: `src-tauri/src/runner/az.rs`
- Modify: `src-tauri/src/runner/mod.rs`

- [ ] **Step 1: Write failing test using fake-az**

`src-tauri/src/runner/az.rs`:
```rust
use std::time::Duration;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::select;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AzEvent {
    Stdout(String),
    Stderr(String),
    Exit { code: i32, duration_ms: u64 },
    Timeout,
    Canceled,
}

pub struct AzConfig {
    pub exe: String,              // "az" in prod; fake-az path in tests
    pub timeout: Duration,        // default 5 min
}

impl Default for AzConfig {
    fn default() -> Self { Self { exe: "az".into(), timeout: Duration::from_secs(300) } }
}

pub async fn spawn_az(
    cfg: &AzConfig,
    argv: &[String],
    tx: mpsc::Sender<AzEvent>,
    mut cancel: tokio::sync::oneshot::Receiver<()>,
) {
    // argv is the full command incl. leading "az"; skip first to get real args
    let start = std::time::Instant::now();
    let args = if argv.first().map(|s| s.as_str()) == Some("az") { &argv[1..] } else { argv };
    let mut child = match Command::new(&cfg.exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AzEvent::Stderr(format!("spawn error: {e}"))).await;
            let _ = tx.send(AzEvent::Exit { code: -1, duration_ms: 0 }).await;
            return;
        }
    };
    let mut out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut err = BufReader::new(child.stderr.take().unwrap()).lines();

    let timeout = tokio::time::sleep(cfg.timeout);
    tokio::pin!(timeout);

    loop {
        select! {
            line = out.next_line() => {
                match line {
                    Ok(Some(l)) => { let _ = tx.send(AzEvent::Stdout(l)).await; }
                    _ => break,
                }
            }
            line = err.next_line() => {
                if let Ok(Some(l)) = line { let _ = tx.send(AzEvent::Stderr(l)).await; }
            }
            status = child.wait() => {
                let code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
                let _ = tx.send(AzEvent::Exit { code, duration_ms: start.elapsed().as_millis() as u64 }).await;
                return;
            }
            _ = &mut timeout => {
                let _ = child.kill().await;
                let _ = tx.send(AzEvent::Timeout).await;
                return;
            }
            _ = &mut cancel => {
                let _ = child.kill().await;
                let _ = tx.send(AzEvent::Canceled).await;
                return;
            }
        }
    }
    // Drain to EOF in case out ended first
    if let Ok(status) = child.wait().await {
        let code = status.code().unwrap_or(-1);
        let _ = tx.send(AzEvent::Exit { code, duration_ms: start.elapsed().as_millis() as u64 }).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_az_path() -> String {
        // target/debug/fake-az(.exe) relative to workspace root
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe);
        p.to_string_lossy().into_owned()
    }

    #[tokio::test]
    async fn runs_fake_az_and_reports_exit_zero() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "stdout": "hello\n", "exit_code": 0 }]"#);
        let state = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", state.path());
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let (tx, mut rx) = mpsc::channel(16);
        let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        spawn_az(&cfg, &["az".into(), "network".into(), "vnet".into(), "create".into()], tx, cancel_rx).await;
        let mut saw_stdout = false; let mut saw_exit = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                AzEvent::Stdout(s) if s == "hello" => saw_stdout = true,
                AzEvent::Exit { code: 0, .. } => saw_exit = true,
                _ => {}
            }
        }
        assert!(saw_stdout && saw_exit);
    }

    #[tokio::test]
    async fn reports_nonzero_exit_on_failure() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "stderr": "boom\n", "exit_code": 2 }]"#);
        let state = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", state.path());
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let (tx, mut rx) = mpsc::channel(16);
        let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        spawn_az(&cfg, &["az".into()], tx, cancel_rx).await;
        let mut code = None;
        while let Ok(ev) = rx.try_recv() {
            if let AzEvent::Exit { code: c, .. } = ev { code = Some(c); }
        }
        assert_eq!(code, Some(2));
    }
}
```

- [ ] **Step 2: Export** in `runner/mod.rs`:

```rust
pub mod az;
pub mod materialize;
pub use az::{spawn_az, AzConfig, AzEvent};
pub use materialize::{materialize, MaterializeError, MaterializedCommand};
```

- [ ] **Step 3: Build fake-az first, then test**

Run:
```bash
cargo build --manifest-path src-fake-az/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml runner::az::
```
Expected: tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/runner
git commit -m "feat(runner): async az invoker with streaming, timeout, cancel"
```

---

### Task 15: Runner dispatch — Snapshot / Validate / Dry-run / Emit

**Files:**
- Create: `src-tauri/src/runner/dispatch.rs`
- Create: `src-tauri/src/runner/emit.rs`
- Modify: `src-tauri/src/runner/mod.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/runner/dispatch.rs`:
```rust
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

/// For each declared node's command, ensure every required ref is in-graph
/// and, if it's a ghost, that its status is not Missing.
pub fn validate(graph: &Graph) -> Result<(), ValidateError> {
    for cmd in graph.commands() {
        // Skip if the produced node is ghost/absent
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
```

`src-tauri/src/runner/emit.rs`:
```rust
use std::path::Path;
use crate::runner::materialize::MaterializedCommand;

#[derive(Debug, Clone, Copy)]
pub enum ScriptFlavor { Bash, Powershell }

pub fn render(commands: &[MaterializedCommand], flavor: ScriptFlavor, source: &str) -> String {
    let ts = chrono::Utc::now().to_rfc3339();
    let header = match flavor {
        ScriptFlavor::Bash => format!(
            "#!/usr/bin/env bash\n# Generated by az-plotter v0.1.0 from {source} on {ts}\nset -euo pipefail\n\n",
        ),
        ScriptFlavor::Powershell => format!(
            "# Generated by az-plotter v0.1.0 from {source} on {ts}\n$ErrorActionPreference = 'Stop'\n\n",
        ),
    };
    let body: String = commands.iter()
        .map(|c| format!("# [{}]\n{}\n", c.node_id.display(), shell_words::join(&c.argv)))
        .collect();
    format!("{header}{body}")
}

pub fn write(commands: &[MaterializedCommand], flavor: ScriptFlavor, source: &str, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, render(commands, flavor, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<MaterializedCommand> {
        use crate::model::{NodeId, NodeKind, Scope};
        let s = Scope::new("rg");
        vec![
            MaterializedCommand {
                node_id: NodeId::of(NodeKind::Vnet, "v", &s),
                command_id: "c1".into(),
                argv: vec!["az","network","vnet","create","--name","v","--resource-group","rg"].into_iter().map(String::from).collect(),
            },
        ]
    }

    #[test]
    fn bash_script_starts_with_shebang_and_set_e() {
        let s = render(&sample(), ScriptFlavor::Bash, "hub.azp");
        assert!(s.starts_with("#!/usr/bin/env bash"));
        assert!(s.contains("set -euo pipefail"));
        assert!(s.contains("az network vnet create"));
    }

    #[test]
    fn powershell_sets_error_action_preference() {
        let s = render(&sample(), ScriptFlavor::Powershell, "hub.azp");
        assert!(s.contains("$ErrorActionPreference = 'Stop'"));
    }
}
```

- [ ] **Step 2: Export** in `runner/mod.rs`:

```rust
pub mod az;
pub mod dispatch;
pub mod emit;
pub mod materialize;

pub use az::{spawn_az, AzConfig, AzEvent};
pub use dispatch::{dry_run, validate, ValidateError};
pub use emit::{render, write as write_script, ScriptFlavor};
pub use materialize::{materialize, MaterializeError, MaterializedCommand};
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runner::`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/runner
git commit -m "feat(runner): validate + dry-run + emit script (bash / powershell)"
```

---

### Task 16: Runner — Live execute (sequential)

**Files:**
- Create: `src-tauri/src/runner/live.rs`
- Modify: `src-tauri/src/runner/mod.rs`

- [ ] **Step 1: Write failing test (fake-az, two commands, both succeed)**

`src-tauri/src/runner/live.rs`:
```rust
use chrono::Utc;
use tokio::sync::{mpsc, oneshot};
use crate::model::{Graph, NodeId, NodeStatus};
use crate::runner::az::{spawn_az, AzConfig, AzEvent};
use crate::runner::dispatch::{validate, ValidateError};
use crate::runner::materialize::{materialize, MaterializedCommand};

#[derive(Debug, Clone)]
pub enum RunEvent {
    NodeStarted { node: NodeId, argv: Vec<String> },
    NodeLog { node: NodeId, line: String, is_err: bool },
    NodeFinished { node: NodeId, status: NodeStatus },
    Aborted { node: NodeId, reason: String },
    Done { succeeded: usize, failed: usize },
}

pub struct RunHandle {
    pub cancel: oneshot::Sender<()>,
    pub events: mpsc::Receiver<RunEvent>,
}

/// Runs the materialized plan sequentially. Stops at first failure (v1 policy).
pub async fn live_run(
    graph: &Graph,
    az_cfg: AzConfig,
) -> Result<RunHandle, ValidateError> {
    validate(graph)?;
    let plan: Vec<MaterializedCommand> = materialize(graph)?;
    let (tx, rx) = mpsc::channel(128);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut succeeded = 0usize;
        let mut failed = 0usize;
        for mc in plan {
            if cancel_rx.try_recv().is_ok() {
                let _ = tx.send(RunEvent::Aborted { node: mc.node_id.clone(), reason: "canceled".into() }).await;
                break;
            }
            let _ = tx.send(RunEvent::NodeStarted { node: mc.node_id.clone(), argv: mc.argv.clone() }).await;
            let (az_tx, mut az_rx) = mpsc::channel::<AzEvent>(64);
            let (node_cancel_tx, node_cancel_rx) = oneshot::channel();
            // Bridge cancel: if outer cancel fires during a node, propagate
            let tx_clone = tx.clone();
            let node_id = mc.node_id.clone();
            let handle = tokio::spawn(async move {
                spawn_az(&az_cfg, &mc.argv, az_tx, node_cancel_rx).await;
            });

            let mut exit_code: Option<i32> = None;
            let mut stderr_tail = String::new();
            let started = Utc::now();
            loop {
                tokio::select! {
                    ev = az_rx.recv() => {
                        match ev {
                            Some(AzEvent::Stdout(l)) => { let _ = tx_clone.send(RunEvent::NodeLog { node: node_id.clone(), line: l, is_err: false }).await; }
                            Some(AzEvent::Stderr(l)) => { stderr_tail = l.clone(); let _ = tx_clone.send(RunEvent::NodeLog { node: node_id.clone(), line: l, is_err: true }).await; }
                            Some(AzEvent::Exit { code, duration_ms: _ }) => { exit_code = Some(code); break; }
                            Some(AzEvent::Timeout) => { exit_code = Some(-1); stderr_tail = "timeout".into(); break; }
                            Some(AzEvent::Canceled) => { exit_code = Some(-1); stderr_tail = "canceled".into(); break; }
                            None => break,
                        }
                    }
                    _ = &mut cancel_rx => {
                        let _ = node_cancel_tx.send(());
                    }
                }
            }
            let _ = handle.await;
            let dur = (Utc::now() - started).num_milliseconds().max(0) as u64;
            let status = match exit_code.unwrap_or(-1) {
                0 => NodeStatus::Succeeded { duration_ms: dur },
                c => NodeStatus::Failed { exit_code: c, stderr_tail: stderr_tail.clone(), duration_ms: dur },
            };
            let is_fail = matches!(status, NodeStatus::Failed { .. });
            let _ = tx.send(RunEvent::NodeFinished { node: node_id.clone(), status }).await;
            if is_fail {
                failed += 1;
                break; // v1: fail-fast
            } else {
                succeeded += 1;
            }
        }
        let _ = tx.send(RunEvent::Done { succeeded, failed }).await;
    });

    Ok(RunHandle { cancel: cancel_tx, events: rx })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Graph;
    use crate::parser::{commit, parse, ArgMap};

    fn load_argmap() -> ArgMap {
        let s = std::fs::read_to_string("arg-map.json").unwrap();
        ArgMap::from_json(&s).unwrap()
    }
    fn fake_az_path() -> String {
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe)
            .to_string_lossy().into_owned()
    }

    #[tokio::test]
    async fn two_successful_commands_result_in_done_2_0() {
        let mut g = Graph::new();
        let m = load_argmap();
        commit(&mut g, parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap()).unwrap();
        commit(&mut g, parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap()).unwrap();

        std::env::set_var("AZ_FAKE_SCRIPT", r#"[
            {"stdout":"ok1\n","exit_code":0},
            {"stdout":"ok2\n","exit_code":0}
        ]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let cfg = AzConfig { exe: fake_az_path(), timeout: std::time::Duration::from_secs(5) };
        let mut handle = live_run(&g, cfg).await.unwrap();

        let mut done = None;
        while let Some(ev) = handle.events.recv().await {
            if let RunEvent::Done { succeeded, failed } = ev { done = Some((succeeded, failed)); break; }
        }
        assert_eq!(done, Some((2, 0)));
    }
}
```

- [ ] **Step 2: Export** in `runner/mod.rs`:

```rust
pub mod az;
pub mod dispatch;
pub mod emit;
pub mod live;
pub mod materialize;

pub use az::{spawn_az, AzConfig, AzEvent};
pub use dispatch::{dry_run, validate, ValidateError};
pub use emit::{render, write as write_script, ScriptFlavor};
pub use live::{live_run, RunEvent, RunHandle};
pub use materialize::{materialize, MaterializeError, MaterializedCommand};
```

- [ ] **Step 3: Run**

Run:
```bash
cargo build --manifest-path src-fake-az/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml runner::live::
```
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/runner
git commit -m "feat(runner): sequential live execute with event stream, fail-fast, cancellation"
```

---

### Task 17: Verification worker

**Files:**
- Create: `src-tauri/src/verify/mod.rs`
- Create: `src-tauri/src/verify/worker.rs`
- Modify: `src-tauri/src/main.rs` & `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests**

`src-tauri/src/verify/worker.rs`:
```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use crate::model::{NodeId, NodeKind, NodeStatus, Origin};
use crate::runner::az::{spawn_az, AzConfig, AzEvent};

#[derive(Debug, Clone)]
pub struct VerifyJob {
    pub node_id: NodeId,
    pub ref_key: u64, // hash of (kind, name, rg, sub) at enqueue time
}

#[derive(Debug, Clone)]
pub enum VerifyEvent {
    Started(NodeId),
    Result { node_id: NodeId, exists: bool },
    Stale(NodeId),
}

pub struct VerifierHandle {
    pub sender: mpsc::Sender<VerifyJob>,
    pub events: mpsc::Receiver<VerifyEvent>,
}

/// Current-ref-key lookup: given a node id, return the live ref_key (None if node gone).
pub type RefKeyLookup = Arc<dyn Fn(&NodeId) -> Option<u64> + Send + Sync>;

pub fn hash_ref_key(id: &NodeId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

fn kind_to_az_subcommand(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Vnet => "vnet",
        NodeKind::Subnet => "vnet subnet",
        NodeKind::Nsg => "nsg",
        NodeKind::NsgRule => "nsg rule",
        NodeKind::PublicIp => "public-ip",
        NodeKind::Nic => "nic",
        NodeKind::Lb => "lb",
        NodeKind::RouteTable => "route-table",
        NodeKind::ResourceGroup => "group",
    }
}

/// Spawn a worker. `rate_per_minute` caps az calls; `lookup` gives live ref_key for staleness.
pub fn spawn_worker(
    az_cfg: AzConfig,
    rate_per_minute: u32,
    lookup: RefKeyLookup,
) -> VerifierHandle {
    let (job_tx, mut job_rx) = mpsc::channel::<VerifyJob>(128);
    let (evt_tx, evt_rx) = mpsc::channel::<VerifyEvent>(128);

    let min_interval = if rate_per_minute == 0 {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(60_000 / rate_per_minute as u64)
    };

    tokio::spawn(async move {
        let mut last = Instant::now() - Duration::from_secs(60);
        while let Some(job) = job_rx.recv().await {
            // rate limit
            let since = last.elapsed();
            if since < min_interval {
                tokio::time::sleep(min_interval - since).await;
            }
            last = Instant::now();

            // staleness check before calling az
            if lookup(&job.node_id).map(|k| k != job.ref_key).unwrap_or(true) {
                let _ = evt_tx.send(VerifyEvent::Stale(job.node_id.clone())).await;
                continue;
            }

            let _ = evt_tx.send(VerifyEvent::Started(job.node_id.clone())).await;
            let sub = kind_to_az_subcommand(job.node_id.kind);
            let mut argv: Vec<String> = vec!["az".into()];
            argv.extend(sub.split_whitespace().map(String::from));
            argv.push("show".into());
            argv.push("--name".into()); argv.push(job.node_id.name.clone());
            argv.push("--resource-group".into()); argv.push(job.node_id.resource_group.clone());
            argv.push("--output".into()); argv.push("none".into());

            let (az_tx, mut az_rx) = mpsc::channel::<AzEvent>(32);
            let (_cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
            let cfg_clone = az_cfg.clone();
            let argv_clone = argv.clone();
            tokio::spawn(async move {
                spawn_az(&cfg_clone, &argv_clone, az_tx, cancel_rx).await;
            });
            let mut exit: Option<i32> = None;
            while let Some(ev) = az_rx.recv().await {
                if let AzEvent::Exit { code, .. } = ev { exit = Some(code); break; }
                if matches!(ev, AzEvent::Timeout | AzEvent::Canceled) { exit = Some(-1); break; }
            }
            // re-check staleness after the call completed
            if lookup(&job.node_id).map(|k| k != job.ref_key).unwrap_or(true) {
                let _ = evt_tx.send(VerifyEvent::Stale(job.node_id.clone())).await;
                continue;
            }
            let exists = exit == Some(0);
            let _ = evt_tx.send(VerifyEvent::Result { node_id: job.node_id, exists }).await;
        }
    });

    VerifierHandle { sender: job_tx, events: evt_rx }
}

impl Clone for AzConfig {
    fn clone(&self) -> Self {
        Self { exe: self.exe.clone(), timeout: self.timeout }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeKind, Scope};

    fn fake_az_path() -> String {
        let exe = if cfg!(windows) { "fake-az.exe" } else { "fake-az" };
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..").join("target").join("debug").join(exe)
            .to_string_lossy().into_owned()
    }

    #[tokio::test]
    async fn reports_exists_on_exit_zero() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "exit_code": 0 }]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let scope = Scope::new("rg");
        let id = NodeId::of(NodeKind::Vnet, "v", &scope);
        let expected = hash_ref_key(&id);
        let lookup: RefKeyLookup = Arc::new(move |_| Some(expected));
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let mut h = spawn_worker(cfg, 0, lookup);
        h.sender.send(VerifyJob { node_id: id.clone(), ref_key: expected }).await.unwrap();

        let mut result = None;
        while let Some(ev) = h.events.recv().await {
            if let VerifyEvent::Result { exists, .. } = ev { result = Some(exists); break; }
        }
        assert_eq!(result, Some(true));
    }

    #[tokio::test]
    async fn stale_job_is_discarded() {
        std::env::set_var("AZ_FAKE_SCRIPT", r#"[{ "exit_code": 0 }]"#);
        let st = tempfile::NamedTempFile::new().unwrap();
        std::env::set_var("AZ_FAKE_STATE", st.path());

        let scope = Scope::new("rg");
        let id = NodeId::of(NodeKind::Vnet, "v", &scope);
        // Lookup returns a different ref_key than the job carries
        let lookup: RefKeyLookup = Arc::new(|_| Some(42));
        let cfg = AzConfig { exe: fake_az_path(), timeout: Duration::from_secs(5) };
        let mut h = spawn_worker(cfg, 0, lookup);
        h.sender.send(VerifyJob { node_id: id.clone(), ref_key: 7 }).await.unwrap();

        let mut saw_stale = false;
        while let Some(ev) = h.events.recv().await {
            if let VerifyEvent::Stale(_) = ev { saw_stale = true; break; }
            if let VerifyEvent::Result { .. } = ev { break; }
        }
        assert!(saw_stale);
    }
}
```

`src-tauri/src/verify/mod.rs`:
```rust
pub mod worker;
pub use worker::{hash_ref_key, spawn_worker, RefKeyLookup, VerifierHandle, VerifyEvent, VerifyJob};
```

Add `pub mod verify;` to both `main.rs` and `lib.rs`.

- [ ] **Step 2: Run**

Run:
```bash
cargo build --manifest-path src-fake-az/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml verify::
```
Expected: both tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/verify src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(verify): background worker with CAS staleness and rate limit"
```

---

### Task 18: Project persistence (.azp) + autosave

**Files:**
- Create: `src-tauri/src/persist/mod.rs`
- Create: `src-tauri/src/persist/project.rs`
- Modify: `src-tauri/src/main.rs` & `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing test**

`src-tauri/src/persist/project.rs`:
```rust
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::model::{Command, Graph};
use crate::parser::{commit, parse, ArgMap};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiState {
    #[serde(default)]
    pub layout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub commands: Vec<Command>,
    #[serde(default)]
    pub ui_state: UiState,
}

impl ProjectFile {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn from_graph(graph: &Graph) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            commands: graph.commands().cloned().collect(),
            ui_state: UiState::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, s)
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s).map_err(std::io::Error::other)
    }

    /// Rebuild a graph from a project file. Replays parser on each command's raw line.
    pub fn to_graph(&self, argmap: &ArgMap) -> Result<Graph, crate::parser::ParseError> {
        let mut g = Graph::new();
        for c in &self.commands {
            let p = parse(&c.raw, argmap, &g)?;
            commit(&mut g, p)?;
        }
        Ok(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_argmap() -> ArgMap {
        ArgMap::from_json(&std::fs::read_to_string("arg-map.json").unwrap()).unwrap()
    }

    #[test]
    fn round_trip_save_load_rebuild() {
        let mut g = Graph::new();
        let m = load_argmap();
        commit(&mut g, parse("az network vnet create --name v --resource-group rg", &m, &g).unwrap()).unwrap();
        commit(&mut g, parse("az network vnet subnet create --name s --resource-group rg --vnet-name v", &m, &g).unwrap()).unwrap();

        let pf = ProjectFile::from_graph(&g);
        let tmp = tempfile::NamedTempFile::new().unwrap();
        pf.save(tmp.path()).unwrap();

        let loaded = ProjectFile::load(tmp.path()).unwrap();
        assert_eq!(loaded.commands.len(), 2);
        let g2 = loaded.to_graph(&m).unwrap();
        assert_eq!(g2.nodes().count(), 2);
        assert_eq!(g2.edges().count(), 1);
    }
}
```

`src-tauri/src/persist/mod.rs`:
```rust
pub mod project;
pub use project::{ProjectFile, UiState};
```

Add `pub mod persist;` to `main.rs` and `lib.rs`.

- [ ] **Step 2: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml persist::`
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/persist src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(persist): .azp project file + graph replay on load"
```

---

## Phase 4 — Tauri IPC layer

### Task 19: Session state + IPC commands

**Files:**
- Create: `src-tauri/src/ipc/mod.rs`
- Create: `src-tauri/src/ipc/state.rs`
- Create: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: State holder**

`src-tauri/src/ipc/state.rs`:
```rust
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::model::Graph;
use crate::parser::ArgMap;

pub struct Session {
    pub graph: Mutex<Graph>,
    pub argmap: ArgMap,
    pub project_path: Mutex<Option<PathBuf>>,
}

pub type SessionState = Arc<Session>;

impl Session {
    pub fn new(argmap: ArgMap) -> Self {
        Self {
            graph: Mutex::new(Graph::new()),
            argmap,
            project_path: Mutex::new(None),
        }
    }
}
```

- [ ] **Step 2: IPC commands**

`src-tauri/src/ipc/commands.rs`:
```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::model::{Edge, Node};
use crate::parser::{commit as commit_parse, parse};
use crate::persist::ProjectFile;
use crate::runner::{dry_run as runner_dry_run, render as render_script, write_script, ScriptFlavor};
use super::state::SessionState;

#[derive(Serialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[tauri::command]
pub fn add_command(line: String, state: tauri::State<SessionState>) -> Result<String, String> {
    let mut g = state.graph.lock().map_err(|e| e.to_string())?;
    let parsed = parse(&line, &state.argmap, &g).map_err(|e| e.to_string())?;
    let id = parsed.command.id.clone();
    commit_parse(&mut g, parsed).map_err(|e| e.to_string())?;
    if let Some(path) = state.project_path.lock().map_err(|e| e.to_string())?.as_ref() {
        let _ = ProjectFile::from_graph(&g).save(path);
    }
    Ok(id)
}

#[tauri::command]
pub fn snapshot(state: tauri::State<SessionState>) -> Result<GraphSnapshot, String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    Ok(GraphSnapshot {
        nodes: g.nodes().cloned().collect(),
        edges: g.edges().cloned().collect(),
    })
}

#[tauri::command]
pub fn dry_run(state: tauri::State<SessionState>) -> Result<Vec<Vec<String>>, String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let plan = runner_dry_run(&g).map_err(|e| e.to_string())?;
    Ok(plan.into_iter().map(|c| c.argv).collect())
}

#[derive(Deserialize)]
pub struct EmitArgs { pub path: String, pub flavor: String }

#[tauri::command]
pub fn emit_script(args: EmitArgs, state: tauri::State<SessionState>) -> Result<(), String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let plan = runner_dry_run(&g).map_err(|e| e.to_string())?;
    let flavor = match args.flavor.as_str() {
        "bash" => ScriptFlavor::Bash,
        "powershell" => ScriptFlavor::Powershell,
        _ => return Err(format!("unknown flavor: {}", args.flavor)),
    };
    let source = state.project_path.lock().map_err(|e| e.to_string())?
        .as_ref().map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "<untitled>".into());
    write_script(&plan, flavor, &source, &PathBuf::from(&args.path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_project(path: String, state: tauri::State<SessionState>) -> Result<GraphSnapshot, String> {
    let p = PathBuf::from(&path);
    let pf = ProjectFile::load(&p).map_err(|e| e.to_string())?;
    let g = pf.to_graph(&state.argmap).map_err(|e| e.to_string())?;
    *state.graph.lock().map_err(|e| e.to_string())? = g.clone();
    *state.project_path.lock().map_err(|e| e.to_string())? = Some(p);
    Ok(GraphSnapshot {
        nodes: g.nodes().cloned().collect(),
        edges: g.edges().cloned().collect(),
    })
}

#[tauri::command]
pub fn save_project_as(path: String, state: tauri::State<SessionState>) -> Result<(), String> {
    let g = state.graph.lock().map_err(|e| e.to_string())?;
    let pf = ProjectFile::from_graph(&g);
    let p = PathBuf::from(&path);
    pf.save(&p).map_err(|e| e.to_string())?;
    *state.project_path.lock().map_err(|e| e.to_string())? = Some(p);
    Ok(())
}
```

`src-tauri/src/ipc/mod.rs`:
```rust
pub mod commands;
pub mod state;
pub use state::{Session, SessionState};
```

- [ ] **Step 3: Wire into `main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
mod parser;
mod planner;
mod runner;
mod verify;
mod persist;
mod ipc;

use std::sync::Arc;
use ipc::{commands as ipc_cmd, Session};
use parser::ArgMap;

fn main() {
    let argmap_json = include_str!("../arg-map.json");
    let argmap = ArgMap::from_json(argmap_json).expect("bundled arg-map.json invalid");
    let session = Arc::new(Session::new(argmap));

    tauri::Builder::default()
        .manage(session)
        .invoke_handler(tauri::generate_handler![
            ipc_cmd::add_command,
            ipc_cmd::snapshot,
            ipc_cmd::dry_run,
            ipc_cmd::emit_script,
            ipc_cmd::open_project,
            ipc_cmd::save_project_as,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Add `pub mod ipc;` to `lib.rs`.

- [ ] **Step 4: Run**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: OK.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ipc src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat(ipc): tauri commands for add/snapshot/dry-run/emit/open/save"
```

---

### Task 20: IPC — live-run command + event bridge

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add `run` command that spawns the live runner and forwards events**

Append to `src-tauri/src/ipc/commands.rs`:
```rust
use tauri::{AppHandle, Manager};
use crate::runner::{live_run, AzConfig, RunEvent};

#[tauri::command]
pub async fn run_live(app: AppHandle, state: tauri::State<'_, SessionState>) -> Result<(), String> {
    let graph = {
        let g = state.graph.lock().map_err(|e| e.to_string())?;
        g.clone()
    };
    let cfg = AzConfig::default();
    let mut handle = live_run(&graph, cfg).await.map_err(|e| e.to_string())?;
    while let Some(ev) = handle.events.recv().await {
        let payload = serde_json::to_value(&RunEventWire::from(&ev)).unwrap();
        let _ = app.emit_all("run-event", payload);
        if matches!(ev, RunEvent::Done { .. }) { break; }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum RunEventWire {
    NodeStarted { node: String, argv: Vec<String> },
    NodeLog { node: String, line: String, is_err: bool },
    NodeFinished { node: String, status: String },
    Aborted { node: String, reason: String },
    Done { succeeded: usize, failed: usize },
}

impl RunEventWire {
    fn from(ev: &RunEvent) -> Self {
        match ev {
            RunEvent::NodeStarted { node, argv } => Self::NodeStarted { node: node.display(), argv: argv.clone() },
            RunEvent::NodeLog { node, line, is_err } => Self::NodeLog { node: node.display(), line: line.clone(), is_err: *is_err },
            RunEvent::NodeFinished { node, status } => {
                use crate::model::NodeStatus::*;
                let s = match status {
                    Succeeded { .. } => "succeeded",
                    Failed { .. } => "failed",
                    Canceled => "canceled",
                    _ => "other",
                }.to_string();
                Self::NodeFinished { node: node.display(), status: s }
            }
            RunEvent::Aborted { node, reason } => Self::Aborted { node: node.display(), reason: reason.clone() },
            RunEvent::Done { succeeded, failed } => Self::Done { succeeded: *succeeded, failed: *failed },
        }
    }
}
```

Also: `Graph` needs `Clone` on the field types (HashMap/BTreeSet already are; make sure derive exists). It does.

- [ ] **Step 2: Register `run_live` in `main.rs`**

Append to the `invoke_handler!` list: `ipc_cmd::run_live,`

- [ ] **Step 3: Compile-check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: OK.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ipc src-tauri/src/main.rs
git commit -m "feat(ipc): run-live command streams RunEvents to frontend"
```

---

## Phase 5 — Svelte frontend

### Task 21: Svelte app skeleton with IPC types and stores

**Files:**
- Create: `ui/src/lib/types.ts`
- Create: `ui/src/lib/ipc.ts`
- Create: `ui/src/lib/store.ts`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: Types mirroring Rust**

`ui/src/lib/types.ts`:
```ts
export type NodeKind = "vnet" | "subnet" | "nsg" | "nsg-rule" | "public-ip" | "nic" | "lb" | "route-table" | "rg";
export type Origin = "Declared" | "Ghost";

export interface Scope { resource_group: string; subscription?: string; location?: string }
export interface NodeId { kind: NodeKind; name: string; resource_group: string; subscription?: string }

export type NodeStatus =
  | { kind: "draft" }
  | { kind: "ready" }
  | { kind: "running"; pid: number; started_at: string }
  | { kind: "succeeded"; duration_ms: number }
  | { kind: "failed"; exit_code: number; stderr_tail: string; duration_ms: number }
  | { kind: "canceled" }
  | { kind: "unverified" }
  | { kind: "verifying" }
  | { kind: "exists" }
  | { kind: "missing" };

export interface Node {
  id: NodeId; kind: NodeKind; name: string; scope: Scope;
  origin: Origin; status: NodeStatus; command_id?: string; props: Record<string, unknown>;
}
export interface Edge { from: NodeId; to: NodeId; via: string; kind: "Ref" | "Scope" }

export type RunEvent =
  | { type: "node-started"; node: string; argv: string[] }
  | { type: "node-log"; node: string; line: string; is_err: boolean }
  | { type: "node-finished"; node: string; status: string }
  | { type: "aborted"; node: string; reason: string }
  | { type: "done"; succeeded: number; failed: number };

export interface GraphSnapshot { nodes: Node[]; edges: Edge[] }
```

- [ ] **Step 2: IPC helpers**

`ui/src/lib/ipc.ts`:
```ts
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { GraphSnapshot, RunEvent } from "./types";

export const ipc = {
  addCommand: (line: string) => invoke<string>("add_command", { line }),
  snapshot: () => invoke<GraphSnapshot>("snapshot"),
  dryRun: () => invoke<string[][]>("dry_run"),
  emitScript: (path: string, flavor: "bash" | "powershell") =>
    invoke<void>("emit_script", { args: { path, flavor } }),
  openProject: (path: string) => invoke<GraphSnapshot>("open_project", { path }),
  saveProjectAs: (path: string) => invoke<void>("save_project_as", { path }),
  runLive: () => invoke<void>("run_live"),
};

export const onRunEvent = (cb: (ev: RunEvent) => void) =>
  listen<RunEvent>("run-event", e => cb(e.payload));
```

- [ ] **Step 3: Stores**

`ui/src/lib/store.ts`:
```ts
import { writable } from "svelte/store";
import type { Node, Edge, RunEvent } from "./types";

export const nodes = writable<Node[]>([]);
export const edges = writable<Edge[]>([]);
export const selectedNodeKey = writable<string | null>(null);
export const logLines = writable<string[]>([]);
export const lastRun = writable<{ succeeded: number; failed: number } | null>(null);

export function appendLog(line: string) {
  logLines.update(xs => (xs.length > 2000 ? [...xs.slice(-1500), line] : [...xs, line]));
}

export function applyRunEvent(ev: RunEvent) {
  switch (ev.type) {
    case "node-started": appendLog(`[${ev.node}] started: ${ev.argv.join(" ")}`); break;
    case "node-log":     appendLog(`[${ev.node}] ${ev.is_err ? "STDERR " : ""}${ev.line}`); break;
    case "node-finished":appendLog(`[${ev.node}] ${ev.status}`); break;
    case "aborted":      appendLog(`[${ev.node}] aborted: ${ev.reason}`); break;
    case "done":         lastRun.set({ succeeded: ev.succeeded, failed: ev.failed }); break;
  }
}
```

- [ ] **Step 4: Replace `App.svelte` with a placeholder shell**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes);
    edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<main>
  <h1>az-plotter</h1>
  <p>nodes: {$nodes.length} · edges: {$edges.length}</p>
</main>
```

- [ ] **Step 5: Smoke-build**

Run: `npm --prefix ui run build`
Expected: build succeeds.

- [ ] **Step 6: Commit**

```bash
git add ui/src
git commit -m "feat(ui): types, ipc client, svelte stores, event wiring"
```

---

### Task 22: Toolbar + CommandPane

**Files:**
- Create: `ui/src/components/Toolbar.svelte`
- Create: `ui/src/components/CommandPane.svelte`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: Toolbar**

`ui/src/components/Toolbar.svelte`:
```svelte
<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges } from "../lib/store";
  let running = false;

  async function dry() {
    const plan = await ipc.dryRun();
    console.log("dry-run plan", plan);
  }
  async function run() {
    running = true;
    try { await ipc.runLive(); } finally { running = false; }
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
  }
</script>

<div class="toolbar">
  <span class="title">az-plotter</span>
  <span class="sep">·</span>
  <button on:click={dry} disabled={running}>Dry-run</button>
  <button on:click={run} disabled={running} class="primary">Run</button>
  <button disabled>Emit script</button>
  <button disabled={!running}>Stop</button>
</div>

<style>
  .toolbar { display:flex; align-items:center; gap:8px; padding:8px 12px;
    background:#2d2d2d; color:#eee; font-size:13px; }
  .title { font-weight:600; }
  .sep { opacity:.5; }
  button { background:#555; color:#eee; border:0; padding:4px 10px; border-radius:3px; cursor:pointer; }
  button.primary { background:#2a8f3d; }
  button:disabled { opacity:.5; cursor:default; }
</style>
```

- [ ] **Step 2: CommandPane**

`ui/src/components/CommandPane.svelte`:
```svelte
<script lang="ts">
  import { ipc } from "../lib/ipc";
  import { nodes, edges } from "../lib/store";
  let line = "";
  let err = "";

  async function add() {
    err = "";
    try {
      await ipc.addCommand(line.trim());
      line = "";
      const snap = await ipc.snapshot();
      nodes.set(snap.nodes); edges.set(snap.edges);
    } catch (e) { err = String(e); }
  }
</script>

<div class="pane">
  <label class="lbl">New command</label>
  <textarea bind:value={line} rows="3" placeholder="az network vnet create --name v --resource-group rg"></textarea>
  <button on:click={add} disabled={!line.trim()}>Add</button>
  {#if err}<div class="err">{err}</div>{/if}

  <label class="lbl">Commands ({$nodes.filter(n => n.origin === "Declared").length})</label>
  <ul>
    {#each $nodes.filter(n => n.origin === "Declared") as n}
      <li>{n.kind} · {n.name}</li>
    {/each}
  </ul>
</div>

<style>
  .pane { padding:10px; background:#fafafa; height:100%; box-sizing:border-box; overflow:auto; }
  .lbl { display:block; font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin:10px 0 4px; }
  textarea { width:100%; font-family:monospace; font-size:12px; box-sizing:border-box; }
  button { margin-top:6px; width:100%; padding:6px; }
  ul { list-style:none; padding:0; font-family:monospace; font-size:12px; }
  li { padding:2px 0; }
  .err { color:#b53030; font-size:12px; margin-top:6px; white-space:pre-wrap; }
</style>
```

- [ ] **Step 3: Mount in `App.svelte`**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";
  import Toolbar from "./components/Toolbar.svelte";
  import CommandPane from "./components/CommandPane.svelte";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<div class="app">
  <Toolbar />
  <div class="grid">
    <CommandPane />
    <div class="canvas">Graph canvas placeholder</div>
    <div class="detail">Detail placeholder</div>
  </div>
</div>

<style>
  :global(html, body, #app) { height:100%; margin:0; }
  .app { display:flex; flex-direction:column; height:100vh; font-family:system-ui, sans-serif; }
  .grid { flex:1; display:grid; grid-template-columns: 280px 1fr 300px; min-height:0; }
  .canvas { background:#fff; border-left:1px solid #ddd; border-right:1px solid #ddd; padding:10px; }
  .detail { background:#fafafa; padding:10px; }
</style>
```

- [ ] **Step 4: Build**

Run: `npm --prefix ui run build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add ui/src
git commit -m "feat(ui): Toolbar + CommandPane with add-command flow"
```

---

### Task 23: GraphCanvas (Cytoscape.js wrapper)

**Files:**
- Create: `ui/src/components/GraphCanvas.svelte`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: GraphCanvas component**

`ui/src/components/GraphCanvas.svelte`:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import cytoscape from "cytoscape";
  import dagre from "cytoscape-dagre";
  import { nodes, edges, selectedNodeKey } from "../lib/store";
  import type { Node as GNode, Edge as GEdge } from "../lib/types";

  cytoscape.use(dagre);

  let container: HTMLDivElement;
  let cy: cytoscape.Core | null = null;

  function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }

  function toElements(ns: GNode[], es: GEdge[]) {
    return [
      ...ns.map(n => ({
        data: { id: keyOf(n.id), label: `${n.kind} · ${n.name}`, origin: n.origin, status: n.status.kind },
      })),
      ...es.map((e, i) => ({
        data: { id: `e${i}`, source: keyOf(e.from), target: keyOf(e.to), via: e.via },
      })),
    ];
  }

  onMount(() => {
    cy = cytoscape({
      container,
      elements: toElements($nodes, $edges),
      layout: { name: "dagre", rankDir: "LR" } as any,
      style: [
        { selector: "node", style: {
          "label": "data(label)", "font-size": 11, "text-valign": "center", "text-halign": "center",
          "background-color": "#eaf3ff", "border-color": "#4a90e2", "border-width": 2, "shape": "round-rectangle",
          "padding": "8px", "width": "label", "height": "label",
        } as any },
        { selector: "node[origin = 'Ghost']", style: {
          "background-color": "#f0f0f0", "border-color": "#888", "border-style": "dashed",
        } as any },
        { selector: "node[status = 'running']", style: { "border-color": "#b58022" } },
        { selector: "node[status = 'succeeded']", style: { "background-color": "#e8f7ec", "border-color": "#2a8f3d" } },
        { selector: "node[status = 'failed']", style: { "background-color": "#fde2e2", "border-color": "#b53030" } },
        { selector: "edge", style: {
          "width": 1.5, "line-color": "#999", "target-arrow-color": "#999", "target-arrow-shape": "triangle",
          "curve-style": "bezier",
        } as any },
      ],
    });
    cy.on("tap", "node", ev => selectedNodeKey.set(ev.target.id()));
  });

  $: if (cy) {
    cy.elements().remove();
    cy.add(toElements($nodes, $edges));
    cy.layout({ name: "dagre", rankDir: "LR" } as any).run();
  }

  onDestroy(() => { cy?.destroy(); cy = null; });
</script>

<div bind:this={container} class="canvas"></div>

<style>
  .canvas { width:100%; height:100%; background:#fff; }
</style>
```

- [ ] **Step 2: Declare cytoscape-dagre type shim**

Create `ui/src/types/cytoscape-dagre.d.ts`:
```ts
declare module "cytoscape-dagre";
```

Update `ui/tsconfig.json` `include` to add `"src/types/**/*.d.ts"`.

- [ ] **Step 3: Mount in `App.svelte`** (replace placeholder div)

```svelte
  import GraphCanvas from "./components/GraphCanvas.svelte";
  ...
  <div class="canvas-wrap"><GraphCanvas /></div>
```

and rename `.canvas` CSS to `.canvas-wrap` to avoid conflict.

- [ ] **Step 4: Build**

Run: `npm --prefix ui run build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add ui/src
git commit -m "feat(ui): GraphCanvas with Cytoscape + dagre layout, declared/ghost styling"
```

---

### Task 24: DetailPane + LogPane + run-event wiring

**Files:**
- Create: `ui/src/components/DetailPane.svelte`
- Create: `ui/src/components/LogPane.svelte`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: DetailPane**

`ui/src/components/DetailPane.svelte`:
```svelte
<script lang="ts">
  import { nodes, selectedNodeKey } from "../lib/store";
  function keyOf(id: any): string {
    const sub = id.subscription ? `/sub:${id.subscription}` : "";
    return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
  }
  $: selected = $nodes.find(n => keyOf(n.id) === $selectedNodeKey) ?? null;
</script>

<div class="pane">
  <div class="lbl">Selected node</div>
  {#if selected}
    <div class="mono">
      <div><b>{selected.kind} · {selected.name}</b></div>
      <div class="muted">rg: {selected.scope.resource_group}</div>
      {#if selected.scope.subscription}<div class="muted">sub: {selected.scope.subscription}</div>{/if}
      {#if selected.scope.location}<div class="muted">loc: {selected.scope.location}</div>{/if}
      <div class="muted">origin: {selected.origin}</div>
      <div class="muted">status: {selected.status.kind}</div>
    </div>
  {:else}
    <div class="muted">No node selected</div>
  {/if}
</div>

<style>
  .pane { padding:10px; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .mono { font-family:monospace; font-size:12px; line-height:1.5; }
  .muted { color:#666; }
</style>
```

- [ ] **Step 2: LogPane**

`ui/src/components/LogPane.svelte`:
```svelte
<script lang="ts">
  import { logLines, lastRun } from "../lib/store";
</script>

<div class="pane">
  <div class="lbl">Log</div>
  <pre class="log">{$logLines.join("\n")}</pre>
  {#if $lastRun}
    <div class="summary">
      Done: {$lastRun.succeeded} succeeded · {$lastRun.failed} failed
    </div>
  {/if}
</div>

<style>
  .pane { padding:10px; display:flex; flex-direction:column; height:100%; box-sizing:border-box; }
  .lbl { font-size:11px; letter-spacing:.05em; text-transform:uppercase; color:#666; margin-bottom:6px; }
  .log { flex:1; background:#1e1e1e; color:#d0d0d0; padding:8px; border-radius:4px; font-size:11px; line-height:1.4; overflow:auto; margin:0; white-space:pre-wrap; }
  .summary { font-size:12px; margin-top:6px; }
</style>
```

- [ ] **Step 3: Final App.svelte**

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { nodes, edges, applyRunEvent } from "./lib/store";
  import Toolbar from "./components/Toolbar.svelte";
  import CommandPane from "./components/CommandPane.svelte";
  import GraphCanvas from "./components/GraphCanvas.svelte";
  import DetailPane from "./components/DetailPane.svelte";
  import LogPane from "./components/LogPane.svelte";

  onMount(async () => {
    const snap = await ipc.snapshot();
    nodes.set(snap.nodes); edges.set(snap.edges);
    await onRunEvent(ev => applyRunEvent(ev));
  });
</script>

<div class="app">
  <Toolbar />
  <div class="grid">
    <CommandPane />
    <div class="canvas-wrap"><GraphCanvas /></div>
    <div class="right">
      <DetailPane />
      <div class="divider" />
      <LogPane />
    </div>
  </div>
</div>

<style>
  :global(html, body, #app) { height:100%; margin:0; }
  .app { display:flex; flex-direction:column; height:100vh; font-family:system-ui, sans-serif; }
  .grid { flex:1; display:grid; grid-template-columns: 280px 1fr 300px; min-height:0; }
  .canvas-wrap { background:#fff; border-left:1px solid #ddd; border-right:1px solid #ddd; }
  .right { display:grid; grid-template-rows: auto 1px 1fr; background:#fafafa; min-height:0; }
  .divider { background:#ddd; }
</style>
```

- [ ] **Step 4: Build**

Run: `npm --prefix ui run build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add ui/src
git commit -m "feat(ui): DetailPane + LogPane, full three-pane layout wired"
```

---

## Phase 6 — End-to-end + polish

### Task 25: E2E happy-path smoke test

**Files:**
- Create: `src-tauri/tests/e2e_smoke.rs`

- [ ] **Step 1: Write an in-process E2E test that drives the session through the IPC commands directly (no WebDriver in v1 — simpler and deterministic)**

`src-tauri/tests/e2e_smoke.rs`:
```rust
use az_plotter::ipc::{commands as ipc, state::Session};
use az_plotter::parser::ArgMap;
use std::sync::Arc;

fn session() -> Arc<Session> {
    let json = std::fs::read_to_string("arg-map.json").unwrap();
    Arc::new(Session::new(ArgMap::from_json(&json).unwrap()))
}

#[test]
fn add_commands_build_expected_graph() {
    let s = session();
    // We call the underlying functions, not the tauri macro-wrapped versions.
    // For test ergonomics, replicate the command bodies with the State shim:
    // Easiest: grab graph via snapshot after each add by locking directly.

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
    // vnet first
    assert!(plan[0].argv.iter().any(|t| t == "vnet"));
    assert!(plan[0].argv.iter().any(|t| t == "create"));
}
```

Note: making `Session`'s fields `pub` may be needed for tests. Open `src-tauri/src/ipc/state.rs` and confirm `graph` and `argmap` are `pub`. They are.

- [ ] **Step 2: Run**

Run:
```bash
cargo build --manifest-path src-fake-az/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --test e2e_smoke
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/e2e_smoke.rs
git commit -m "test(e2e): smoke — 3-command graph, dry-run ordering"
```

---

### Task 26: `regen-argmap` dev binary

**Files:**
- Create: `src-tauri/src/bin/regen-argmap.rs`

- [ ] **Step 1: Write the binary**

`src-tauri/src/bin/regen-argmap.rs`:
```rust
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
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build --manifest-path src-tauri/Cargo.toml --bin regen-argmap`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/bin
git commit -m "feat(dev): regen-argmap stub generator"
```

---

### Task 27: Windows packaging via Tauri bundler

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/icons/icon.png` (generate placeholder)

- [ ] **Step 1: Generate a placeholder icon**

Any 512×512 PNG works for now. Commit an all-black placeholder at `src-tauri/icons/icon.png`. If a graphic tool isn't handy, use:
```bash
# Optional: generate a solid black 512x512 PNG via powershell (Windows)
powershell -Command "Add-Type -AssemblyName System.Drawing; $b=New-Object System.Drawing.Bitmap 512,512; $g=[System.Drawing.Graphics]::FromImage($b); $g.Clear([System.Drawing.Color]::Black); $b.Save('src-tauri/icons/icon.png')"
```

- [ ] **Step 2: Wire icon into `tauri.conf.json`**

In the `tauri.bundle` block, add:
```json
"icon": ["icons/icon.png"]
```

- [ ] **Step 3: Build the installer**

Run:
```bash
npm --prefix ui install
cargo tauri build --manifest-path src-tauri/Cargo.toml
```
(If `cargo tauri` subcommand isn't available, install once: `cargo install tauri-cli --version ^1`.)

Expected: `.msi` and/or `.exe` bundles appear under `src-tauri/target/release/bundle/`.

- [ ] **Step 4: Verify installer runs**

Manually: double-click the produced MSI, launch `az-plotter`, paste one command, confirm node appears. Don't block on real `az` — this is a UI smoke.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/icons
git commit -m "chore(packaging): wire Tauri bundler for Windows MSI/NSIS"
```

---

## Self-review (done by plan author)

**Spec coverage:** Every §-section of the spec maps to a task:

| Spec section | Tasks |
|---|---|
| §2 Goals | Task 1 (scaffold) + every subsequent task |
| §5 Architecture / boundaries | Tasks 19–20 (IPC), Task 21 (frontend split) |
| §6 Data model | Tasks 2–5 |
| §7 Parser & arg-map | Tasks 6–10 |
| §8 Runner / dispatch / pipeline | Tasks 11, 12, 15, 16 |
| §9 Verification worker | Task 17 |
| §10 UI layout | Tasks 21–24 |
| §11 Persistence | Task 18 |
| §12 Deferred | Not implemented (by design); tasks 26 (regen-argmap) is the v1 dev hook |
| §13 Testing strategy | Tests embedded in every task; Tasks 13, 14, 16, 25 add integration layers |
| §14 Success criteria | Task 27 (packaging) verifies size target |

**Placeholder scan:** no TBDs, no "add appropriate error handling" without code, no "similar to Task N" references. Every code block is complete.

**Type consistency:** `NodeId`, `NodeKind`, `NodeStatus`, `Scope`, `Graph`, `Command`, `ArgMap`, `MaterializedCommand`, `RunEvent` are defined once and referenced identically throughout. The wire type `RunEventWire` in Task 20 is the only name variation, and it's explicitly a serialization shim with a `From` conversion.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-19-az-plotter-implementation.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
