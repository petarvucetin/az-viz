<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, onRunEvent } from "./lib/ipc";
  import { appState } from "./lib/store.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import AuthBanner from "./components/AuthBanner.svelte";
  import CommandPane from "./components/CommandPane.svelte";
  import GraphCanvas from "./components/GraphCanvas.svelte";
  import DetailPane from "./components/DetailPane.svelte";
  import LogPane from "./components/LogPane.svelte";

  let logHeight = $state(180);
  let rightWidth = $state(300);

  onMount(() => {
    let unlisten: (() => void) | null = null;
    (async () => {
      const snap = await ipc.snapshot();
      appState.applySnapshot(snap);
      unlisten = await onRunEvent(ev => appState.applyRunEvent(ev));
    })();
    return () => { if (unlisten) unlisten(); };
  });

  function startResize(e: PointerEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startH = logHeight;
    const onMove = (ev: PointerEvent) => {
      const delta = startY - ev.clientY;
      const next = Math.max(80, Math.min(window.innerHeight - 200, startH + delta));
      logHeight = next;
    };
    const onUp = () => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
    };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  }

  function startRightResize(e: PointerEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = rightWidth;
    const onMove = (ev: PointerEvent) => {
      const delta = startX - ev.clientX;
      const next = Math.max(200, Math.min(window.innerWidth - 420, startW + delta));
      rightWidth = next;
    };
    const onUp = () => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
    };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  }
</script>

<div class="app">
  <Toolbar />
  <AuthBanner />
  <div class="main" style="grid-template-columns: 280px 1fr 5px {rightWidth}px;">
    <CommandPane />
    <div class="canvas-wrap"><GraphCanvas /></div>
    <div
      class="v-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize detail panel"
      onpointerdown={startRightResize}
    ></div>
    <DetailPane />
  </div>
  <div
    class="resizer"
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize log pane"
    onpointerdown={startResize}
  ></div>
  <div class="log-wrap" style="height: {logHeight}px">
    <LogPane />
  </div>
</div>

<style>
  :global(html, body, #app) { height:100%; margin:0; }
  .app { display:flex; flex-direction:column; height:100vh; font-family:system-ui, sans-serif; }
  .main { flex:1; display:grid; min-height:0; min-width:0; }
  .canvas-wrap { background:#fff; border-left:1px solid #ddd; min-height:0; min-width:0; }
  .v-resizer {
    background:#ddd; cursor:ew-resize;
    border-left:1px solid #ccc; border-right:1px solid #ccc;
  }
  .v-resizer:hover { background:#b8cfe8; }
  .resizer {
    height:5px; background:#ddd; cursor:ns-resize;
    border-top:1px solid #ccc; border-bottom:1px solid #ccc;
  }
  .resizer:hover { background:#b8cfe8; }
  .log-wrap { background:#fafafa; min-height:80px; overflow:hidden; }
</style>
