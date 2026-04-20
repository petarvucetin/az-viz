import { ipc } from "./ipc";
import { appState } from "./store.svelte";

export async function startAzLogin(): Promise<void> {
  if (appState.authInProgress) return;
  appState.authInProgress = true;
  try {
    await ipc.azLogin();
    appState.authRequired = false;
    const pending = appState.pendingAuthAction;
    appState.pendingAuthAction = null;
    if (pending) {
      try {
        if (pending.kind === "verify") await ipc.verifyNode(pending.logicalKey);
        else await ipc.executeNode(pending.logicalKey);
        const snap = await ipc.snapshot();
        appState.applySnapshot(snap);
      } catch { /* wrapper surfaces via lastError */ }
    }
  } catch { /* wrapper surfaces via lastError */ }
  finally { appState.authInProgress = false; }
}

export async function cancelAzLogin(): Promise<void> {
  try { await ipc.azLoginCancel(); } catch { /* no-op */ }
}

export function dismissAuthBanner(): void {
  appState.authRequired = false;
  appState.pendingAuthAction = null;
}
