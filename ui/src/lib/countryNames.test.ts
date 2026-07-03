import { describe, expect, it } from "vitest";
import { countryName } from "./countryNames";

describe("countryName", () => {
  it("resolves Argentina's real code (3) from country_pointers.csv", () => {
    expect(countryName(3)).toBe("Argentina");
  });

  it("resolves a few other real, spot-checkable codes", () => {
    expect(countryName(0x16)).toBe("España");
    expect(countryName(0x1e)).toBe("Inglaterra");
    expect(countryName(0x39)).toBe("Uruguay");
  });

  it("falls back to the raw numeric code for an unknown byte", () => {
    expect(countryName(255)).toBe("255");
  });
});
