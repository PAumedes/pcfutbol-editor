import { afterEach, describe, expect, it } from "vitest";
import { charmapStatus, detectGameDir, hasTauriBackend, loadPkf, openDbc } from "./ipc";
import { mockDbc, mockTeamIndex } from "./mocks/dbc";

declare global {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  interface Window {
    __TAURI__?: unknown;
  }
}

afterEach(() => {
  delete (window as Window).__TAURI__;
});

describe("hasTauriBackend", () => {
  it("is false in a plain browser session (no __TAURI__ global)", () => {
    expect(hasTauriBackend()).toBe(false);
  });

  it("is true once __TAURI__ is present", () => {
    (window as Window).__TAURI__ = {};
    expect(hasTauriBackend()).toBe(true);
  });
});

describe("ipc mock fallback (no Tauri backend)", () => {
  it("loadPkf falls back to the mock team index", async () => {
    const result = await loadPkf("EQ003003.PKF");
    expect(result).toEqual(mockTeamIndex);
  });

  it("openDbc falls back to the mock Dbc", async () => {
    const result = await openDbc("EQ979013.DBC");
    expect(result).toEqual(mockDbc);
  });

  it("returned mock objects are copies, not shared references", async () => {
    const first = await openDbc("EQ979013.DBC");
    first.team.shortName = "MUTATED";
    const second = await openDbc("EQ979013.DBC");
    expect(second.team.shortName).toBe("BOCA");
  });

  it("detectGameDir returns a mock path", async () => {
    const result = await detectGameDir();
    expect(typeof result).toBe("string");
  });

  it("charmapStatus reports loaded with no missing glyphs in the mock", async () => {
    const result = await charmapStatus();
    expect(result.loaded).toBe(true);
    expect(result.missingGlyphs).toEqual([]);
  });
});
