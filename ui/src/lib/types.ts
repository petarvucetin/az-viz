export type NodeKind = "vnet" | "subnet" | "nsg" | "nsg-rule" | "public-ip" | "nic" | "lb" | "route-table" | "vnet-gateway" | "local-gateway" | "vpn-connection" | "vnet-peering" | "dns-resolver" | "private-dns-zone" | "private-dns-link" | "rg";
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

export type VarBody =
  | { mode: "command"; argv: string[] }
  | { mode: "literal"; value: string }
  | { mode: "unset" };

export interface Variable {
  name: string;
  body: VarBody;
  origin: "Declared" | "Ghost";
  resolved: string | null;
}

export interface Group {
  id: string;
  title: string;
  command_ids: string[];
}

export type BatchAddResult =
  | { kind: "command"; id: string }
  | { kind: "section"; title: string }
  | { kind: "duplicate"; line_index: number; line: string; produces: string }
  | { kind: "error"; line_index: number; line: string; message: string };

export type RunEvent =
  | { type: "node-started"; node: string; argv: string[] }
  | { type: "node-log"; node: string; line: string; is_err: boolean }
  | { type: "node-finished"; node: string; status: string }
  | { type: "aborted"; node: string; reason: string }
  | { type: "done"; succeeded: number; failed: number }
  | { type: "auth-required"; triggered_by: "verify" | "execute"; logical_key: string }
  | { type: "login-log"; line: string; is_err: boolean }
  | { type: "login-finished"; ok: boolean };

export interface GraphSnapshot {
  nodes: Node[];
  edges: Edge[];
  variables: Variable[];
  /** logical key of a produced node → variable names the producing command references */
  var_consumers: Record<string, string[]>;
  groups: Group[];
  /** group id → logical node keys in the group, in declaration order */
  group_nodes: Record<string, string[]>;
}
