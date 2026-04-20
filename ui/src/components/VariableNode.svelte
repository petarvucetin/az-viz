<script lang="ts">
  import { Handle, Position, type NodeProps, type Node } from "@xyflow/svelte";

  type VarData = {
    name: string;
    resolved: string | null;
    origin: "Declared" | "Ghost";
    logicalKey: string;
    selectedDirect?: boolean;
  };

  type VarNode = Node<VarData, "variable">;

  let { data }: NodeProps<VarNode> = $props();

  let hasValue = $derived(data.resolved !== null && data.resolved !== undefined);
  let isGhost = $derived(data.origin === "Ghost");
</script>

<Handle type="target" position={Position.Top} />

<div class="var-node" class:has-value={hasValue} class:ghost={isGhost} class:selected={data.selectedDirect}>
  {#if hasValue}
    <span class="check" title={data.resolved ?? ""}>✓</span>
  {/if}
  <span class="pill">var</span>
  <div class="name">${data.name}</div>
</div>

<Handle type="source" position={Position.Bottom} />

<style>
  .var-node {
    position: relative;
    width: 100%; height: 100%;
    box-sizing: border-box;
    padding: 8px 10px 6px;
    display: flex; flex-direction: column;
    text-align: center;
    background: linear-gradient(135deg, #fff7ed, #fed7aa);
    border: 1.5px dashed #fb923c;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(154, 52, 18, 0.12);
    color: #7c2d12;
    font-family: system-ui, sans-serif;
  }
  .var-node.has-value {
    border-style: solid;
    border-width: 2.5px;
    border-color: #2a8f3d;
  }
  .var-node.ghost { opacity: 0.75; }
  .var-node.selected { border-color: #0b2447; border-width: 3px; }

  .check {
    position: absolute; top: 3px; right: 6px;
    color: #2a8f3d; font-weight: 700; font-size: 14px;
  }
  .pill {
    align-self: flex-start;
    font-size: 9px; font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    background: #fff7ed; color: #9a3412;
    border: 1px solid #fb923c;
    text-transform: lowercase;
    letter-spacing: .04em;
    margin-bottom: 4px;
  }
  .name {
    font-family: monospace;
    font-weight: 700; font-size: 13px;
    color: #7c2d12;
    word-break: break-all;
  }
</style>
