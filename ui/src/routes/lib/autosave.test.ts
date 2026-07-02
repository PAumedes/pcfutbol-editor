import { describe, expect, it, vi } from "vitest";
import {
  clearAutosave,
  createMemoryStore,
  debounce,
  loadAutosave,
  saveAutosave,
} from "./autosave";
import type { Project } from "../../lib/model";
import { mockDbc } from "../../lib/mocks/dbc";

const project: Project = { dbcs: [mockDbc], gameDir: "/games/apertura" };

describe("autosave", () => {
  it("round-trips a project through save/load", () => {
    const store = createMemoryStore();
    saveAutosave(store, project, new Date("2026-07-01T00:00:00Z"));
    const loaded = loadAutosave(store);
    expect(loaded?.project).toEqual(project);
    expect(loaded?.savedAt).toBe("2026-07-01T00:00:00.000Z");
  });

  it("returns null when nothing has been saved", () => {
    const store = createMemoryStore();
    expect(loadAutosave(store)).toBeNull();
  });

  it("returns null (not a throw) for corrupt stored JSON", () => {
    const store = createMemoryStore();
    store.setItem("pcf-editor:autosave:project", "{not json");
    expect(loadAutosave(store)).toBeNull();
  });

  it("returns null for well-formed JSON missing required fields", () => {
    const store = createMemoryStore();
    store.setItem("pcf-editor:autosave:project", JSON.stringify({ foo: "bar" }));
    expect(loadAutosave(store)).toBeNull();
  });

  it("clearAutosave removes the saved entry", () => {
    const store = createMemoryStore();
    saveAutosave(store, project);
    clearAutosave(store);
    expect(loadAutosave(store)).toBeNull();
  });
});

describe("debounce", () => {
  it("only calls the wrapped function once after the delay, with the last args", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced(1);
    debounced(2);
    debounced(3);
    expect(fn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith(3);
    vi.useRealTimers();
  });
});
