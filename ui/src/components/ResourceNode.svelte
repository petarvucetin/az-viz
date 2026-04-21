<script lang="ts">
  import { Handle, Position, type NodeProps, type Node } from "@xyflow/svelte";
  import { cidrToRange } from "../lib/cidr";

  type ResourceData = {
    kind: string;
    name: string;
    origin: string;
    status: string;
    cidr?: string;
    range?: string;
    extraProps?: Array<[string, string]>;
    logicalKey: string;
    context: string;
    selectedDirect?: boolean;
    blocked?: boolean;
  };

  type ResourceNode = Node<ResourceData, "resource">;

  let { data, ...rest }: NodeProps<ResourceNode> = $props();

  let countSuffix = $derived.by(() => {
    if (!data.cidr) return "";
    const r = cidrToRange(data.cidr);
    return r ? ` (${r.count})` : "";
  });

  function truncate(s: string, max = 40): string {
    return s.length > max ? s.slice(0, max - 1) + "\u2026" : s;
  }

  let statusClass = $derived(`status-${data.status}`);
  let originClass = $derived(data.origin === "Ghost" ? "origin-ghost" : "origin-declared");
</script>

<Handle type="target" position={Position.Top} />

<div
  class="azn {statusClass} {originClass}"
  class:selected={data.selectedDirect}
  class:blocked={data.blocked}
  data-ctx={data.context}
  title={data.blocked ? "cannot execute — parent not declared" : undefined}
>
  {#if data.status === "succeeded" || data.status === "exists"}
    <span class="azn-check" title="deployed">✓</span>
  {/if}
  <span class="azn-pill" data-k={data.kind}>{data.kind}</span>
  <div class="azn-name">{data.name}</div>
  {#if data.cidr}
    <div class="azn-cidr">{data.cidr}{countSuffix}</div>
  {/if}
  {#if data.range}
    <div class="azn-range">{data.range}</div>
  {/if}
  {#if data.extraProps}
    {#each data.extraProps.slice(0, 3) as [k, v]}
      <div class="azn-prop"><span class="azn-pk">{k}:</span> {truncate(v)}</div>
    {/each}
  {/if}
</div>

<Handle type="source" position={Position.Bottom} />

<style>
  .azn {
    font-family: var(--app-ui-font, system-ui, sans-serif);
    width: 100%; height: 100%;
    box-sizing: border-box;
    padding: 6px 10px;
    line-height: 1.3;
    display: flex;
    flex-direction: column;
    text-align: center;
    background: linear-gradient(135deg, #f0f7ff, #cfe3fb);
    border: 1.5px dashed #4a90e2;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(11, 36, 71, 0.15);
    color: #0b2447;
  }
  .azn[data-ctx="dns"] { background: linear-gradient(135deg, #faf5ff, #e9d5ff); }

  .azn.origin-ghost { border-color: #888; border-style: dashed; }
  .azn.status-running    { border-color: #b58022; border-style: dashed; }
  .azn.status-succeeded  { border-color: #2a8f3d; border-style: solid; border-width: 2.5px; }
  .azn.status-failed     { border-color: #b53030; border-style: solid; }
  .azn.status-exists     { border-color: #2a8f3d; border-style: solid; border-width: 2.5px; }
  .azn.status-missing    { border-color: #ff8c1a; border-style: dashed; }
  .azn.status-verifying  { border-color: #b58022; border-style: dashed; }
  .azn.selected          { border-color: #0b2447; border-width: 3px; }
  .azn.blocked           { opacity: 0.4; filter: grayscale(0.9); }
  .azn.blocked .azn-pill { filter: grayscale(1); }

  .azn-pill {
    align-self: flex-start;
    margin-bottom: 6px;
    font-size: 9px; font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    background: #f3f4f6; color: #374151;
    border: 1px solid #9ca3af;
    text-transform: lowercase;
    letter-spacing: .04em;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    box-shadow: 0 1px 2px rgba(11, 36, 71, 0.15);
  }
  .azn-pill[data-k="vnet"]          { background:#e0f2fe; color:#0369a1; border-color:#0ea5e9; }
  .azn-pill[data-k="subnet"]        { background:#dcfce7; color:#15803d; border-color:#22c55e; }
  .azn-pill[data-k="nsg"]           { background:#fef3c7; color:#92400e; border-color:#f59e0b; }
  .azn-pill[data-k="nsg-rule"]      { background:#ffedd5; color:#9a3412; border-color:#f97316; }
  .azn-pill[data-k="public-ip"]     { background:#cffafe; color:#0e7490; border-color:#06b6d4; }
  .azn-pill[data-k="nic"]           { background:#f3e8ff; color:#6b21a8; border-color:#a855f7; }
  .azn-pill[data-k="lb"]            { background:#fce7f3; color:#9d174d; border-color:#ec4899; }
  .azn-pill[data-k="route-table"]   { background:#fef9c3; color:#854d0e; border-color:#eab308; }
  .azn-pill[data-k="vnet-gateway"]  { background:#e0e7ff; color:#3730a3; border-color:#6366f1; }
  .azn-pill[data-k="local-gateway"] { background:#ccfbf1; color:#115e59; border-color:#14b8a6; }
  .azn-pill[data-k="vpn-connection"]{ background:#ffe4e6; color:#9f1239; border-color:#f43f5e; }
  .azn-pill[data-k="vnet-peering"]  { background:#ecfccb; color:#3f6212; border-color:#84cc16; }
  .azn-pill[data-k="dns-resolver"]  { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }
  .azn-pill[data-k="private-dns-zone"] { background:#f5f3ff; color:#4c1d95; border-color:#7c3aed; }
  .azn-pill[data-k="private-dns-link"] { background:#ede9fe; color:#5b21b6; border-color:#8b5cf6; }

  .azn { position: relative; }
  .azn-check {
    position: absolute;
    top: -8px;
    right: -8px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #2a8f3d;
    color: #fff;
    font-weight: 900;
    font-size: 13px;
    line-height: 20px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(11, 36, 71, 0.3);
    pointer-events: none;
  }
  .azn-name { font-weight: 700; font-size: 13px; color: #0b2447; letter-spacing: -0.01em; text-align: center; word-break: break-all; }
  .azn-cidr { color: #c9184a; font-size: 11px; font-variant-numeric: tabular-nums; margin-top: 2px; }
  .azn-range { color: #444; font-size: 10px; font-variant-numeric: tabular-nums; }
  .azn-prop { color: #555; font-size: 10px; margin-top: 1px; }
  .azn-pk { color: #888; }
</style>
