import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import type { BatchAddResult, GraphSnapshot, RunEvent, NodeStatus } from "./types";
import { appState } from "./store.svelte";

async function withErrorStore<T>(p: Promise<T>): Promise<T> {
  try { appState.lastError = null; return await p; }
  catch (e) {
    const s = String(e);
    // "not_logged_in" is surfaced via AuthBanner, not as a generic error toast.
    appState.lastError = s === "not_logged_in" ? null : s;
    throw e;
  }
}

export const ipc = {
  addCommand: (line: string) => invoke<string>("add_command", { line }),
  addCommandsBatch: (lines: string[]) =>
    invoke<BatchAddResult[]>("add_commands_batch", { lines }),
  snapshot: () => invoke<GraphSnapshot>("snapshot"),
  dryRun: () => invoke<string[][]>("dry_run"),
  emitScript: (path: string, flavor: "bash" | "powershell") =>
    invoke<void>("emit_script", { args: { path, flavor } }),
  openProject: (path: string) => invoke<GraphSnapshot>("open_project", { path }),
  saveProjectAs: (path: string) => invoke<void>("save_project_as", { path }),
  runLive: () => invoke<void>("run_live"),
  removeCommand: (id: string) =>
    withErrorStore(invoke<void>("remove_command", { id })),
  clearAll: () => withErrorStore(invoke<void>("clear_all")),
  setVariableBody: (name: string, body: string) =>
    withErrorStore(invoke<void>("set_variable_body", { args: { name, body } })),
  refreshVariable: (name: string) =>
    withErrorStore(invoke<string | null>("refresh_variable", { name })),
  removeVariable: (name: string) =>
    withErrorStore(invoke<void>("remove_variable", { name })),
  verifyNode: (logicalKey: string) =>
    withErrorStore(invoke<NodeStatus>("verify_node", { logicalKey })),
  executeNode: (logicalKey: string) =>
    withErrorStore(invoke<void>("execute_node", { logicalKey })),
  azLogin: () => withErrorStore(invoke<void>("az_login")),
  azLoginCancel: () => invoke<void>("az_login_cancel"),
};

export const onRunEvent = (cb: (ev: RunEvent) => void) =>
  listen<RunEvent>("run-event", e => cb(e.payload));
