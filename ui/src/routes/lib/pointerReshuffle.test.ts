import { describe, expect, it } from "vitest";
import { detectPointerReshuffle, PHOTO_IMPORT_ORDER_WARNING } from "./pointerReshuffle";

describe("detectPointerReshuffle", () => {
  it("detects no warning when the list is untouched", () => {
    const before = [
      { pointer: 1, playerId: "palermo" },
      { pointer: 2, playerId: "riquelme" },
    ];
    expect(detectPointerReshuffle(before, before, "auto")).toBeNull();
  });

  it("warns when inserting a player mid-list shifts pointer ownership", () => {
    // Before: pointer 1=palermo, 2=riquelme. Insert a new player at slot 1,
    // pushing riquelme from pointer 2 to pointer 3 and putting "nuevo" at 2.
    const before = [
      { pointer: 1, playerId: "palermo" },
      { pointer: 2, playerId: "riquelme" },
    ];
    const after = [
      { pointer: 1, playerId: "palermo" },
      { pointer: 2, playerId: "nuevo" },
      { pointer: 3, playerId: "riquelme" },
    ];
    const warning = detectPointerReshuffle(before, after, "auto");
    expect(warning).not.toBeNull();
    expect(warning?.code).toBe("pointer-reshuffle");
    expect(warning?.affected).toEqual([{ pointer: 2, playerId: "nuevo" }]);
    expect(warning?.message).toMatch(/photos already imported/i);
  });

  it("never warns in PreserveFromFile mode, since pointers aren't reassigned by position", () => {
    const before = [{ pointer: 1, playerId: "palermo" }];
    const after = [{ pointer: 1, playerId: "someone-else" }];
    expect(detectPointerReshuffle(before, after, "preserve_from_file")).toBeNull();
  });

  it("reports every reshuffled pointer, not just the first", () => {
    const before = [
      { pointer: 1, playerId: "a" },
      { pointer: 2, playerId: "b" },
      { pointer: 3, playerId: "c" },
    ];
    const after = [
      { pointer: 1, playerId: "a" },
      { pointer: 2, playerId: "c" },
      { pointer: 3, playerId: "b" },
    ];
    const warning = detectPointerReshuffle(before, after, "auto");
    expect(warning?.affected).toHaveLength(2);
  });
});

describe("PHOTO_IMPORT_ORDER_WARNING", () => {
  it("is a non-empty, user-facing string mentioning photos and pointers", () => {
    expect(PHOTO_IMPORT_ORDER_WARNING.length).toBeGreaterThan(0);
    expect(PHOTO_IMPORT_ORDER_WARNING).toMatch(/photo/i);
    expect(PHOTO_IMPORT_ORDER_WARNING).toMatch(/pointer/i);
  });
});
