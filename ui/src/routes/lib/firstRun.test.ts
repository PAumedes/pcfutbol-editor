import { describe, expect, it, vi } from "vitest";
import { runDetect, runLoadPkf } from "./firstRun";
import { mockTeamIndex } from "../../lib/mocks/dbc";

describe("runDetect", () => {
  it("moves to 'detected' when a game dir is found", async () => {
    const deps = {
      detectGameDir: vi.fn().mockResolvedValue("/games/apertura"),
      loadPkf: vi.fn(),
    };
    const state = await runDetect(deps);
    expect(state).toEqual({ step: "detected", gameDir: "/games/apertura" });
  });

  it("moves to 'not-detected' when nothing is found", async () => {
    const deps = { detectGameDir: vi.fn().mockResolvedValue(null), loadPkf: vi.fn() };
    const state = await runDetect(deps);
    expect(state).toEqual({ step: "not-detected" });
  });

  it("maps a thrown error to a friendly error state, never a raw stack trace", async () => {
    const deps = {
      detectGameDir: vi.fn().mockRejectedValue(new Error("ENOENT")),
      loadPkf: vi.fn(),
    };
    const state = await runDetect(deps);
    expect(state.step).toBe("error");
    if (state.step === "error") {
      expect(state.message).not.toMatch(/at Object\.|\.ts:\d+/); // no stack-trace shape
      expect(state.message.length).toBeGreaterThan(0);
    }
  });
});

describe("runLoadPkf", () => {
  it("loads the team index for a valid folder", async () => {
    const deps = { detectGameDir: vi.fn(), loadPkf: vi.fn().mockResolvedValue(mockTeamIndex) };
    const state = await runLoadPkf(deps, "/games/apertura");
    expect(state).toEqual({
      step: "loaded",
      gameDir: "/games/apertura",
      teamIndex: mockTeamIndex,
    });
  });

  it("treats an empty team index as a friendly error, not a crash", async () => {
    const deps = { detectGameDir: vi.fn(), loadPkf: vi.fn().mockResolvedValue([]) };
    const state = await runLoadPkf(deps, "/not/a/game/dir");
    expect(state.step).toBe("error");
    if (state.step === "error") {
      expect(state.message).toMatch(/no teams were found/i);
    }
  });

  it("wraps a rejected loadPkf in a friendly error state", async () => {
    const deps = {
      detectGameDir: vi.fn(),
      loadPkf: vi.fn().mockRejectedValue(new Error("permission denied")),
    };
    const state = await runLoadPkf(deps, "/games/apertura");
    expect(state.step).toBe("error");
    if (state.step === "error") {
      expect(state.gameDir).toBe("/games/apertura");
      expect(state.message).toMatch(/permission denied/);
    }
  });
});
