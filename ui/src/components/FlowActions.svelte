<script lang="ts">
  /**
   * Inner component that lives inside <SvelteFlow> so it has access to the
   * useSvelteFlow() hook context. Handles fit-to-screen and auto-pan to
   * the selected node.
   */
  import { useSvelteFlow } from "@xyflow/svelte";
  import type { Writable } from "svelte/store";
  import { selectedNodeKey } from "../lib/store";

  export let fitSignal: Writable<number>;

  const { fitView, getNode, getViewport, setCenter } = useSvelteFlow();

  // Fit-to-screen when toolbar button is pressed.
  $: if ($fitSignal > 0) {
    fitView({ padding: 0.1, duration: 300 });
  }

  // Auto-pan to the selected node when selection changes.
  // Multi-prefix VNets use logical key for selection but visual id includes #pN;
  // try logical key first, then fall back to #p0.
  $: if ($selectedNodeKey) {
    const k = $selectedNodeKey;
    // Small tick delay lets Svelte Flow register updated node positions first.
    setTimeout(() => {
      const n = getNode(k) ?? getNode(`${k}#p0`);
      if (!n || n.position == null) return;
      const w = (n.width ?? 0);
      const h = (n.height ?? 0);
      const vp = getViewport();
      setCenter(n.position.x + w / 2, n.position.y + h / 2, { zoom: vp.zoom, duration: 300 });
    }, 0);
  }
</script>
