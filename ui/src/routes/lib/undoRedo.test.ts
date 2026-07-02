import { describe, expect, it } from "vitest";
import { canRedo, canUndo, initUndoRedo, undoRedoReducer } from "./undoRedo";

describe("undoRedoReducer", () => {
  it("starts with empty history", () => {
    const state = initUndoRedo(1);
    expect(canUndo(state)).toBe(false);
    expect(canRedo(state)).toBe(false);
    expect(state.present).toBe(1);
  });

  it("set pushes the previous present onto past and clears future", () => {
    let state = initUndoRedo(1);
    state = undoRedoReducer(state, { type: "set", value: 2 });
    expect(state.present).toBe(2);
    expect(state.past).toEqual([1]);
    expect(state.future).toEqual([]);
  });

  it("undo restores the previous value and moves it to future", () => {
    let state = initUndoRedo(1);
    state = undoRedoReducer(state, { type: "set", value: 2 });
    state = undoRedoReducer(state, { type: "undo" });
    expect(state.present).toBe(1);
    expect(state.future).toEqual([2]);
    expect(state.past).toEqual([]);
  });

  it("undo on empty history is a no-op", () => {
    const state = initUndoRedo(1);
    expect(undoRedoReducer(state, { type: "undo" })).toBe(state);
  });

  it("redo restores a value that was undone", () => {
    let state = initUndoRedo(1);
    state = undoRedoReducer(state, { type: "set", value: 2 });
    state = undoRedoReducer(state, { type: "undo" });
    state = undoRedoReducer(state, { type: "redo" });
    expect(state.present).toBe(2);
    expect(state.future).toEqual([]);
    expect(state.past).toEqual([1]);
  });

  it("redo on empty future is a no-op", () => {
    const state = initUndoRedo(1);
    expect(undoRedoReducer(state, { type: "redo" })).toBe(state);
  });

  it("a new set after undo discards the redo future (standard undo/redo semantics)", () => {
    let state = initUndoRedo(1);
    state = undoRedoReducer(state, { type: "set", value: 2 });
    state = undoRedoReducer(state, { type: "set", value: 3 });
    state = undoRedoReducer(state, { type: "undo" }); // present=2, future=[3]
    state = undoRedoReducer(state, { type: "set", value: 4 }); // branches history
    expect(state.present).toBe(4);
    expect(state.future).toEqual([]);
    expect(state.past).toEqual([1, 2]);
  });

  it("reset clears all history", () => {
    let state = initUndoRedo(1);
    state = undoRedoReducer(state, { type: "set", value: 2 });
    state = undoRedoReducer(state, { type: "reset", value: 99 });
    expect(state).toEqual(initUndoRedo(99));
  });

  it("caps history at 100 entries so long sessions don't leak memory", () => {
    let state = initUndoRedo(0);
    for (let i = 1; i <= 150; i++) {
      state = undoRedoReducer(state, { type: "set", value: i });
    }
    expect(state.past.length).toBeLessThanOrEqual(100);
    expect(state.present).toBe(150);
  });
});
