import type { NodeKind } from "./types";

const COLOR = "#4a90e2";
const SW = 1.5;

function svgDataUrl(inner: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="14" height="14" ` +
    `fill="none" stroke="${COLOR}" stroke-width="${SW}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

export const KIND_ICONS: Record<NodeKind, string> = {
  // Diamond with 3 dots (VNet "⟨⋯⟩" vibe)
  "vnet": svgDataUrl(`<path d="M8 1.5 L14.5 8 L8 14.5 L1.5 8 Z"/><circle cx="5.5" cy="8" r="0.7" fill="${COLOR}" stroke="none"/><circle cx="8" cy="8" r="0.7" fill="${COLOR}" stroke="none"/><circle cx="10.5" cy="8" r="0.7" fill="${COLOR}" stroke="none"/>`),
  // 2x2 grid (Subnet)
  "subnet": svgDataUrl(`<rect x="2" y="2" width="5" height="5" rx="0.5"/><rect x="9" y="2" width="5" height="5" rx="0.5"/><rect x="2" y="9" width="5" height="5" rx="0.5"/><rect x="9" y="9" width="5" height="5" rx="0.5"/>`),
  // Heraldic shield (NSG)
  "nsg": svgDataUrl(`<path d="M8 1.5 L14 3.5 V8 Q14 12.5 8 14.5 Q2 12.5 2 8 V3.5 Z"/>`),
  // Shield + arrow (NSG rule)
  "nsg-rule": svgDataUrl(`<path d="M8 1.5 L13.5 3.5 V8 Q13.5 12 8 14 Q2.5 12 2.5 8 V3.5 Z"/><path d="M6 8 L10 8 M8.5 6.5 L10 8 L8.5 9.5"/>`),
  // Circle + outward arrow (Public IP)
  "public-ip": svgDataUrl(`<circle cx="6" cy="10" r="3.5"/><path d="M9 7 L14 2 M11 2 H14 V5"/>`),
  // Ethernet plug (NIC)
  "nic": svgDataUrl(`<rect x="4" y="3" width="8" height="9" rx="1"/><rect x="6" y="12" width="4" height="2.5"/><path d="M6 6 V9 M8 6 V9 M10 6 V9"/>`),
  // Splitting arrows (Load balancer)
  "lb": svgDataUrl(`<path d="M8 2 V8 M8 8 L3 14 M8 8 L13 14"/><path d="M1.5 12 L3 14 L4.5 12 M11.5 12 L13 14 L14.5 12"/>`),
  // Forked arrow (Route table)
  "route-table": svgDataUrl(`<path d="M8 2 V6 M8 6 L3 11 V14 M8 6 L13 11 V14"/><path d="M1.5 12 L3 14 L4.5 12 M11.5 12 L13 14 L14.5 12"/>`),
  // Gateway arrow (VNet Gateway) — bidirectional arrow through a box
  "vnet-gateway": svgDataUrl(`<rect x="2" y="5" width="12" height="6" rx="1"/><path d="M1 8 L5 8 M11 8 L15 8"/><path d="M3 6 L1 8 L3 10 M13 6 L15 8 L13 10"/>`),
  // House silhouette (Local Gateway = on-prem endpoint)
  "local-gateway": svgDataUrl(`<path d="M2 8 L8 3 L14 8 V13 Q14 14 13 14 H3 Q2 14 2 13 Z"/><rect x="7" y="10" width="2" height="4"/>`),
  // Two boxes joined by a lightning-ish squiggle (VPN connection)
  "vpn-connection": svgDataUrl(`<rect x="1" y="6" width="4" height="4" rx="0.5"/><rect x="11" y="6" width="4" height="4" rx="0.5"/><path d="M5 8 L8 6 L8 10 L11 8"/>`),
  // Folder (Resource group)
  "rg": svgDataUrl(`<path d="M1.5 4 H6 L7.5 5.5 H14.5 V13 Q14.5 14 13.5 14 H2.5 Q1.5 14 1.5 13 Z"/>`),
};
