import { describe, expect, it } from "vitest";
import { ATTRIBUTE_ORDER } from "./attributeLabels";

describe("ATTRIBUTE_ORDER", () => {
  it("matches the exact on-disk order from PLAN.md Appendix A (VE,RE,AG,CA,RM,RG,PA,TI,EN,PO)", () => {
    expect(ATTRIBUTE_ORDER.map((a) => a.code)).toEqual([
      "VE",
      "RE",
      "AG",
      "CA",
      "RM",
      "RG",
      "PA",
      "TI",
      "EN",
      "PO",
    ]);
  });

  it("covers exactly the 10 keys of the Attributes model, once each", () => {
    const keys = ATTRIBUTE_ORDER.map((a) => a.key);
    expect(keys.sort()).toEqual(
      [
        "velocidad",
        "resistencia",
        "agresividad",
        "calidad",
        "remate",
        "regate",
        "pase",
        "tiro",
        "entradas",
        "portero",
      ].sort(),
    );
  });
});
