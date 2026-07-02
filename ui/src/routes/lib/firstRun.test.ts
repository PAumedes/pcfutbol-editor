import { describe, expect, it, vi } from "vitest";
import { runDetect, runLoadPkf, runSelectTeam } from "./firstRun";
import { mockDbc, mockTeamIndex } from "../../lib/mocks/dbc";

function noopDeps() {
  return { detectGameDir: vi.fn(), loadPkf: vi.fn(), loadPkfTeam: vi.fn() };
}

describe("runDetect", () => {
  it("moves to 'detected' when a game dir is found", async () => {
    const deps = { ...noopDeps(), detectGameDir: vi.fn().mockResolvedValue("/games/apertura") };
    const state = await runDetect(deps);
    expect(state).toEqual({ step: "detected", gameDir: "/games/apertura" });
  });

  it("moves to 'not-detected' when nothing is found", async () => {
    const deps = { ...noopDeps(), detectGameDir: vi.fn().mockResolvedValue(null) };
    const state = await runDetect(deps);
    expect(state).toEqual({ step: "not-detected" });
  });

  it("maps a thrown error to a friendly error state, never a raw stack trace", async () => {
    const deps = {
      ...noopDeps(),
      detectGameDir: vi.fn().mockRejectedValue(new Error("ENOENT")),
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
  it("loads the team index for a valid folder, reading DBDAT\\EQ003003.PKF under it", async () => {
    const loadPkf = vi.fn().mockResolvedValue(mockTeamIndex);
    const deps = { ...noopDeps(), loadPkf };
    const state = await runLoadPkf(deps, "/games/apertura");
    expect(state).toEqual({
      step: "loaded",
      gameDir: "/games/apertura",
      pkfPath: "/games/apertura/DBDAT/EQ003003.PKF",
      teamIndex: mockTeamIndex,
    });
    expect(loadPkf).toHaveBeenCalledWith("/games/apertura/DBDAT/EQ003003.PKF");
  });

  it("joins a Windows-style folder with backslashes", async () => {
    const loadPkf = vi.fn().mockResolvedValue(mockTeamIndex);
    const deps = { ...noopDeps(), loadPkf };
    const state = await runLoadPkf(deps, "C:\\Games\\Apertura 98-99\\");
    expect(state.step).toBe("loaded");
    if (state.step === "loaded") {
      expect(state.pkfPath).toBe("C:\\Games\\Apertura 98-99\\DBDAT\\EQ003003.PKF");
    }
    expect(loadPkf).toHaveBeenCalledWith("C:\\Games\\Apertura 98-99\\DBDAT\\EQ003003.PKF");
  });

  it("treats an empty team index as a friendly error, not a crash", async () => {
    const deps = { ...noopDeps(), loadPkf: vi.fn().mockResolvedValue([]) };
    const state = await runLoadPkf(deps, "/not/a/game/dir");
    expect(state.step).toBe("error");
    if (state.step === "error") {
      expect(state.message).toMatch(/no teams were found/i);
    }
  });

  it("wraps a rejected loadPkf in a friendly error state", async () => {
    const deps = {
      ...noopDeps(),
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

describe("runSelectTeam", () => {
  const loaded = {
    step: "loaded" as const,
    gameDir: "/games/apertura",
    pkfPath: "/games/apertura/DBDAT/EQ003003.PKF",
    teamIndex: mockTeamIndex,
  };

  it("loads the picked team's full Dbc by pointer", async () => {
    const loadPkfTeam = vi.fn().mockResolvedValue(mockDbc);
    const deps = { ...noopDeps(), loadPkfTeam };
    const state = await runSelectTeam(deps, loaded, 9013);
    expect(loadPkfTeam).toHaveBeenCalledWith(loaded.pkfPath, 9013);
    expect(state).toEqual({ step: "team-loaded", gameDir: loaded.gameDir, dbc: mockDbc });
  });

  it("wraps a rejected loadPkfTeam in a friendly error state", async () => {
    const deps = {
      ...noopDeps(),
      loadPkfTeam: vi.fn().mockRejectedValue(new Error("no team with pointer 42")),
    };
    const state = await runSelectTeam(deps, loaded, 42);
    expect(state.step).toBe("error");
    if (state.step === "error") {
      expect(state.gameDir).toBe(loaded.gameDir);
      expect(state.message).toMatch(/no team with pointer 42/);
    }
  });
});
