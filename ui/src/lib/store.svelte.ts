import type { Node, Edge, GraphSnapshot, RunEvent, Variable } from "./types";

export type PendingAuthAction =
  | { kind: "verify"; logicalKey: string }
  | { kind: "execute"; logicalKey: string };

class AppState {
  nodes = $state<Node[]>([]);
  edges = $state<Edge[]>([]);
  variables = $state<Variable[]>([]);
  /** Map from logical node key → variable names referenced by its producer command */
  varConsumers = $state<Record<string, string[]>>({});
  selectedNodeKey = $state<string | null>(null);
  logLines = $state<string[]>([]);
  lastRun = $state<{ succeeded: number; failed: number } | null>(null);
  lastError = $state<string | null>(null);
  fitSignal = $state(0);
  layoutSignal = $state(0);

  authRequired = $state(false);
  authInProgress = $state(false);
  pendingAuthAction = $state<PendingAuthAction | null>(null);

  autoCreate = $state(false);

  settings = $state({
    uiFont: "system-ui, sans-serif",
    monoFont: "ui-monospace, Menlo, Consolas, monospace",
    fontSize: 12,
  });

  applySnapshot(snap: GraphSnapshot) {
    this.nodes = snap.nodes;
    this.edges = snap.edges;
    this.variables = snap.variables ?? [];
    this.varConsumers = snap.var_consumers ?? {};
  }

  appendLog(line: string) {
    const ts = new Date().toISOString().replace("T", " ").slice(0, 19);
    const stamped = `${ts} ${line}`;
    this.logLines = this.logLines.length > 2000
      ? [...this.logLines.slice(-1500), stamped]
      : [...this.logLines, stamped];
  }

  applyRunEvent(ev: RunEvent) {
    switch (ev.type) {
      case "node-started": this.appendLog(`[${ev.node}] started: ${ev.argv.join(" ")}`); break;
      case "node-log":     /* per-line output suppressed — started + finished is enough */ break;
      case "node-finished": this.appendLog(`[${ev.node}] ${ev.status}`); break;
      case "aborted":      this.appendLog(`[${ev.node}] aborted: ${ev.reason}`); break;
      case "done":         this.lastRun = { succeeded: ev.succeeded, failed: ev.failed }; break;
      case "auth-required":
        this.authRequired = true;
        this.pendingAuthAction = { kind: ev.triggered_by, logicalKey: ev.logical_key };
        this.appendLog(`[az login] required — triggered by ${ev.triggered_by} of ${ev.logical_key}`);
        break;
      case "login-log":
        this.appendLog(`[az login] ${ev.is_err ? "STDERR " : ""}${ev.line}`);
        break;
      case "login-finished":
        this.appendLog(`[az login] ${ev.ok ? "signed in" : "failed"}`);
        break;
    }
  }
}

export const appState = new AppState();
