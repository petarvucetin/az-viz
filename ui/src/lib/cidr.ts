export interface ParsedCidr { base: number; prefixLen: number }
export interface CidrRange { first: string; last: string; count: number }

const IPV4_OCTET = /^(25[0-5]|2[0-4]\d|1?\d?\d)$/;

function parseIpv4(s: string): number | null {
  const parts = s.split(".");
  if (parts.length !== 4) return null;
  let n = 0;
  for (const p of parts) {
    if (!IPV4_OCTET.test(p)) return null;
    n = (n * 256 + Number(p)) >>> 0;
  }
  return n;
}

function ipv4ToString(n: number): string {
  return [(n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff].join(".");
}

export function parseCidr(s: string): ParsedCidr | null {
  if (typeof s !== "string" || s.includes(":")) return null;
  const slash = s.indexOf("/");
  if (slash < 0) return null;
  const ipPart = s.slice(0, slash);
  const lenPart = s.slice(slash + 1);
  if (!/^\d+$/.test(lenPart)) return null;
  const prefixLen = Number(lenPart);
  if (prefixLen < 0 || prefixLen > 32) return null;
  const base = parseIpv4(ipPart);
  if (base === null) return null;
  const mask = prefixLen === 0 ? 0 : (0xffffffff << (32 - prefixLen)) >>> 0;
  if (((base & mask) >>> 0) !== base) return null; // host bits set
  return { base, prefixLen };
}

export function cidrToRange(s: string): CidrRange | null {
  const p = parseCidr(s);
  if (!p) return null;
  const size = 2 ** (32 - p.prefixLen);
  const last = (p.base + size - 1) >>> 0;
  return { first: ipv4ToString(p.base), last: ipv4ToString(last), count: size };
}

export function cidrContains(outer: string, inner: string | undefined): boolean {
  if (!inner) return false;
  const o = parseCidr(outer);
  if (!o) return false;
  const i = parseCidr(inner);
  if (!i) return false;
  if (i.prefixLen < o.prefixLen) return false;
  const mask = o.prefixLen === 0 ? 0 : (0xffffffff << (32 - o.prefixLen)) >>> 0;
  return ((i.base & mask) >>> 0) === o.base;
}
