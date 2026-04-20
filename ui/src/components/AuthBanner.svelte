<script lang="ts">
  import { appState } from "../lib/store.svelte";
  import { startAzLogin, cancelAzLogin, dismissAuthBanner } from "../lib/auth";

  let pendingLabel = $derived.by(() => {
    const a = appState.pendingAuthAction;
    if (!a) return "";
    return `${a.kind} of ${a.logicalKey}`;
  });
</script>

{#if appState.authRequired}
  <div class="banner" role="alert">
    <span class="icon">⚠</span>
    <span class="msg">
      Not signed in to Azure.
      {#if pendingLabel}<span class="pending">Waiting to retry {pendingLabel}.</span>{/if}
    </span>
    <span class="spacer"></span>
    {#if appState.authInProgress}
      <span class="status">Signing in… see Log pane</span>
      <button class="btn" onclick={cancelAzLogin}>Cancel</button>
    {:else}
      <button class="btn primary" onclick={startAzLogin}>Log in</button>
      <button class="btn" onclick={dismissAuthBanner}>Dismiss</button>
    {/if}
  </div>
{/if}

<style>
  .banner {
    display:flex; align-items:center; gap:8px;
    padding:6px 12px; font-size:12px;
    background:#3a2f1f; color:#fff3d6; border-bottom:1px solid #6b4a1a;
  }
  .icon { color:#ffb020; font-size:14px; }
  .msg { font-weight:500; }
  .pending { margin-left:6px; opacity:.75; font-weight:400; }
  .spacer { flex:1; }
  .status { font-style:italic; opacity:.8; }
  .btn {
    background:#555; color:#eee; border:0; padding:3px 10px;
    border-radius:3px; cursor:pointer; font-size:12px;
  }
  .btn.primary { background:#2a8f3d; }
  .btn:hover { filter:brightness(1.1); }
</style>
