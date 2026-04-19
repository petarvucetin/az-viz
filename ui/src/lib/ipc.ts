import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { GraphSnapshot, RunEvent, NodeStatus } from "./types";
import { lastError } from "./store";

async function withErrorStore<T>(p: Promise<T>): Promise<T> {
  try { lastError.set(null); return await p; }
  catch (e) { lastError.set(String(e)); throw e; }
}

export const ipc = {
  addCommand: (line: string) => invoke<string>("add_command", { line }),
  snapshot: () => invoke<GraphSnapshot>("snapshot"),
  dryRun: () => invoke<string[][]>("dry_run"),
  emitScript: (path: string, flavor: "bash" | "powershell") =>
    invoke<void>("emit_script", { args: { path, flavor } }),
  openProject: (path: string) => invoke<GraphSnapshot>("open_project", { path }),
  saveProjectAs: (path: string) => invoke<void>("save_project_as", { path }),
  runLive: () => invoke<void>("run_live"),
  removeCommand: (id: string) =>
    withErrorStore(invoke<void>("remove_command", { id })),
  verifyNode: (logicalKey: string) =>
    withErrorStore(invoke<NodeStatus>("verify_node", { logicalKey })),
  executeNode: (logicalKey: string) =>
    withErrorStore(invoke<void>("execute_node", { logicalKey })),
};

export const onRunEvent = (cb: (ev: RunEvent) => void) =>
  listen<RunEvent>("run-event", e => cb(e.payload));
