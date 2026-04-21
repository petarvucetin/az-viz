<script lang="ts">
  import { useSvelteFlow } from "@xyflow/svelte";

  // Captures the Svelte Flow instance and hands it to the parent via
  // callback. This component MUST be rendered inside <SvelteFlowProvider>
  // because useSvelteFlow depends on that context. The parent can then
  // call api.updateNode(...) to patch the internal node store directly,
  // which — unlike modifying the bind:nodes array — works mid-drag.
  let { onready }: { onready: (api: ReturnType<typeof useSvelteFlow>) => void } = $props();
  const api = useSvelteFlow();
  onready(api);
</script>
