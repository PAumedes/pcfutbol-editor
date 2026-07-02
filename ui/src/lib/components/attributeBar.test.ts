import { describe, expect, it } from "vitest";
import { attributePercent, clampAttribute } from "./attributeBar";

describe("clampAttribute", () => {
  it("passes through in-range values", () => {
    expect(clampAttribute(50)).toBe(50);
    expect(clampAttribute(0)).toBe(0);
    expect(clampAttribute(99)).toBe(99);
  });

  it("clamps values above 99 down to 99", () => {
    expect(clampAttribute(100)).toBe(99);
    expect(clampAttribute(255)).toBe(99);
  });

  it("clamps negative values up to 0", () => {
    expect(clampAttribute(-1)).toBe(0);
    expect(clampAttribute(-50)).toBe(0);
  });

  it("rounds fractional values", () => {
    expect(clampAttribute(50.6)).toBe(51);
    expect(clampAttribute(50.4)).toBe(50);
  });

  it("treats NaN as 0", () => {
    expect(clampAttribute(NaN)).toBe(0);
  });
});

describe("attributePercent", () => {
  it("maps 0 to 0% and 99 to 100%", () => {
    expect(attributePercent(0)).toBe(0);
    expect(attributePercent(99)).toBe(100);
  });

  it("clamps out-of-range input before converting to percent", () => {
    expect(attributePercent(150)).toBe(100);
    expect(attributePercent(-10)).toBe(0);
  });
});
