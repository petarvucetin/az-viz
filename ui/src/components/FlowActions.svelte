<script lang="ts">
  import { untrack } from "svelte";
  import { useSvelteFlow } from "@xyflow/svelte";
  import { appState } from "../lib/store.svelte";

  const { fitView, getNode, getViewport, setCenter } = useSvelteFlow();

  let lastFitSignal = 0;
  $effect(() => {
    const v = appState.fitSignal;
    if (v !== lastFitSignal && v > 0) {
      lastFitSignal = v;
      untrack(() => fitView({ padding: 0.1, duration: 300 }));
    }
  });

  let lastSelKey: string | null = null;
  $effect(() => {
    const k = appState.selectedNodeKey;
    if (!k || k === lastSelKey) { lastSelKey = k; return; }
    lastSelKey = k;
    untrack(() => {
      setTimeout(() => {
        const n = getNode(k) ?? getNode(`${k}#p0`);
        if (!n || n.position == null) return;
        const w = (n.measured?.width ?? n.width ?? 0);
        const h = (n.measured?.height ?? n.height ?? 0);
        const vp = getViewport();
        setCenter(n.position.x + w / 2, n.position.y + h / 2, { zoom: vp.zoom, duration: 300 });
      }, 0);
    });
  });
</script>
