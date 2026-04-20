import type { Node, Edge, RunEvent } from "./types";

class AppState {
  nodes = $state<Node[]>([]);
  edges = $state<Edge[]>([]);
  selectedNodeKey = $state<string | null>(null);
  logLines = $state<string[]>([]);
  lastRun = $state<{ succeeded: number; failed: number } | null>(null);
  lastError = $state<string | null>(null);
  fitSignal = $state(0);
  layoutSignal = $state(0);

  appendLog(line: string) {
    this.logLines = this.logLines.length > 2000
      ? [...this.logLines.slice(-1500), line]
      : [...this.logLines, line];
  }

  applyRunEvent(ev: RunEvent) {
    switch (ev.type) {
      case "node-started": this.appendLog(`[${ev.node}] started: ${ev.argv.join(" ")}`); break;
      case "node-log":     this.appendLog(`[${ev.node}] ${ev.is_err ? "STDERR " : ""}${ev.line}`); break;
      case "node-finished": this.appendLog(`[${ev.node}] ${ev.status}`); break;
      case "aborted":      this.appendLog(`[${ev.node}] aborted: ${ev.reason}`); break;
      case "done":         this.lastRun = { succeeded: ev.succeeded, failed: ev.failed }; break;
    }
  }
}

export const appState = new AppState();
