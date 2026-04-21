<script lang="ts">
  import type { NodeProps, Node } from "@xyflow/svelte";

  type GroupData = { title: string; logicalKey: string; selectedDirect?: boolean };
  type GroupSFNode = Node<GroupData, "group">;

  let { data }: NodeProps<GroupSFNode> = $props();
</script>

<div class="group-frame" class:selected={data.selectedDirect}>
  <div class="title">{data.title}</div>
</div>

<style>
  .group-frame {
    width: 100%; height: 100%;
    box-sizing: border-box;
    border: 1.5px dashed #6b7280;
    border-radius: 8px;
    background: rgba(203, 213, 225, 0.10);
    position: relative;
    /* Frame is the drag handle for the whole group — children inside it are
       non-draggable. Border + title surface the grabbable area. */
    cursor: grab;
  }
  .group-frame:active { cursor: grabbing; }
  .group-frame.selected { border-color: #0b2447; border-width: 2.5px; }
  .title {
    position: absolute;
    top: -8px; left: 10px;
    background: #fff;
    padding: 0 6px;
    font-family: var(--app-ui-font, system-ui, sans-serif);
    font-size: 11px; font-weight: 700;
    color: #475569;
    letter-spacing: 0.02em;
    pointer-events: auto;
    user-select: text;
  }
</style>
