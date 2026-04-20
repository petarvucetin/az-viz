import type { Node, Edge } from "./types";

export const containerKindFor: Record<string, string> = {
  "subnet": "vnet",
  "nsg-rule": "nsg",
  "private-dns-link": "private-dns-zone",
};

export function keyOf(id: { kind: string; name: string; resource_group: string; subscription?: string }): string {
  const sub = id.subscription ? `/sub:${id.subscription}` : "";
  return `${id.kind}/${id.name}@rg:${id.resource_group}${sub}`;
}

// Find the container-parent key for a node (e.g. a subnet's vnet), or null
// if this kind has no container relationship or no matching edge exists.
export function parentKeyOf(n: Node, edges: Edge[]): string | null {
  const pk = containerKindFor[n.kind];
  if (!pk) return null;
  const childKey = keyOf(n.id);
  for (const e of edges) {
    if (keyOf(e.to) === childKey && e.from.kind === pk) return keyOf(e.from);
  }
  return null;
}

// A node is blocked (cannot execute) if:
//   - it was never declared (origin === "Ghost"), OR
//   - ANY resource it references (incoming edge sources: --vnet-name, --nsg-name,
//     --virtual-network, --zone-name, etc.) is blocked.
// Propagated transitively via fixed-point iteration.
export function computeBlocked(nodes: Node[], edges: Edge[]): Set<string> {
  const allKeys = new Set<string>();
  for (const n of nodes) allKeys.add(keyOf(n.id));

  // refSourcesOf[child] = list of keys this child depends on (incoming edge `from`s).
  const refSourcesOf: Record<string, string[]> = {};
  for (const e of edges) {
    const src = keyOf(e.from);
    const tgt = keyOf(e.to);
    (refSourcesOf[tgt] ??= []).push(src);
  }

  const blocked = new Set<string>();
  for (const n of nodes) {
    if (n.origin === "Ghost") blocked.add(keyOf(n.id));
  }

  let changed = true;
  while (changed) {
    changed = false;
    for (const n of nodes) {
      const k = keyOf(n.id);
      if (blocked.has(k)) continue;
      const refs = refSourcesOf[k];
      if (!refs) continue;
      for (const p of refs) {
        if (!allKeys.has(p) || blocked.has(p)) {
          blocked.add(k);
          changed = true;
          break;
        }
      }
    }
  }

  return blocked;
}
