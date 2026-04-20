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

export const lastError = writable<string | null>(null);

// Incremented by Toolbar to trigger GraphCanvas to fit the viewport.
export const fitSignal = writable<number>(0);
