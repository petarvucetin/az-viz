import { describe, it, expect } from "vitest";
import { parseCidr, cidrToRange, cidrContains } from "./cidr";

describe("parseCidr", () => {
  it("parses a standard /26", () => {
    const r = parseCidr("10.0.0.0/26");
    expect(r).toEqual({ base: 0x0A000000, prefixLen: 26 });
  });
  it("returns null for IPv6", () => {
    expect(parseCidr("::1/128")).toBeNull();
  });
  it("returns null for malformed input", () => {
    expect(parseCidr("not-a-cidr")).toBeNull();
    expect(parseCidr("10.0.0.0")).toBeNull();
    expect(parseCidr("10.0.0.0/33")).toBeNull();
    expect(parseCidr("10.0.0.1/24")).toBeNull(); // host bits set
  });
  it("accepts /0 and /32", () => {
    expect(parseCidr("0.0.0.0/0")).toEqual({ base: 0, prefixLen: 0 });
    expect(parseCidr("192.168.1.1/32")).toEqual({ base: 0xC0A80101, prefixLen: 32 });
  });
});

describe("cidrToRange", () => {
  it("computes /26 range and count", () => {
    expect(cidrToRange("10.0.0.0/26")).toEqual({
      first: "10.0.0.0", last: "10.0.0.63", count: 64,
    });
  });
  it("handles /32 as single IP", () => {
    expect(cidrToRange("192.168.1.1/32")).toEqual({
      first: "192.168.1.1", last: "192.168.1.1", count: 1,
    });
  });
  it("handles /0 as full space", () => {
    const r = cidrToRange("0.0.0.0/0");
    expect(r?.first).toBe("0.0.0.0");
    expect(r?.last).toBe("255.255.255.255");
    expect(r?.count).toBe(4294967296);
  });
  it("returns null for IPv6 or malformed", () => {
    expect(cidrToRange("::1/128")).toBeNull();
    expect(cidrToRange("bogus")).toBeNull();
  });
});

describe("cidrContains", () => {
  it("outer contains inner (subnet in vnet)", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.0.0/27")).toBe(true);
    expect(cidrContains("10.0.0.0/26", "10.0.0.32/27")).toBe(true);
  });
  it("outer does not contain unrelated inner", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.1.0/27")).toBe(false);
  });
  it("outer contains itself", () => {
    expect(cidrContains("10.0.0.0/26", "10.0.0.0/26")).toBe(true);
  });
  it("inner smaller prefix cannot fit in outer larger prefix", () => {
    expect(cidrContains("10.0.0.0/27", "10.0.0.0/26")).toBe(false);
  });
  it("returns false for IPv6 inputs", () => {
    expect(cidrContains("::/0", "::1/128")).toBe(false);
    expect(cidrContains("10.0.0.0/24", "::1/128")).toBe(false);
  });
  it("returns false for missing/malformed input", () => {
    expect(cidrContains("10.0.0.0/24", undefined)).toBe(false);
    expect(cidrContains("10.0.0.0/24", "")).toBe(false);
  });
});
