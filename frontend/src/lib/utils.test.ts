import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fmtRelative } from "./utils";

describe("fmtRelative", () => {
  const now = new Date("2024-01-15T12:00:00Z");

  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(now);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it.each([
    { offset: 0, expected: "just now" },
    { offset: 1000 * 30, expected: "just now" },
    { offset: 1000 * 60 * 30, expected: "30m ago" },
    { offset: 1000 * 60 * 60 * 3, expected: "3h ago" },
    { offset: 1000 * 60 * 60 * 24 * 3, expected: "3d ago" },
    { offset: 1000 * 60 * 60 * 24 * 14, expected: "2w ago" },
    { offset: 1000 * 60 * 60 * 24 * 27, expected: "3w ago" },
    { offset: 1000 * 60 * 60 * 24 * 28, expected: "1mo ago" },
    { offset: 1000 * 60 * 60 * 24 * 35, expected: "1mo ago" },
    { offset: 1000 * 60 * 60 * 24 * 42, expected: "2mo ago" },
    { offset: 1000 * 60 * 60 * 24 * 60, expected: "2mo ago" },
    { offset: 1000 * 60 * 60 * 24 * 70, expected: "3mo ago" },
  ])("formats $offset as $expected", ({ offset, expected }) => {
    const dt = new Date(now.getTime() - offset).toISOString();
    expect(fmtRelative(dt)).toBe(expected);
  });

  it("returns empty string for null/undefined", () => {
    expect(fmtRelative(null)).toBe("");
    expect(fmtRelative(undefined)).toBe("");
  });
});
