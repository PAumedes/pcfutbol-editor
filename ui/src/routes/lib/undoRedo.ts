// Generic, pure undo/redo reducer used by every edit screen.
//
// Deliberately un-clever: a linear past/future stack of full snapshots of
// whatever `T` the screen is editing (a Dbc, a Player, a Project — the
// screens decide). Svelte components dispatch actions into this reducer and
// re-render off the returned state; no framework code lives in here so it's
// trivial to unit test.

export interface UndoRedoState<T> {
  past: T[];
  present: T;
  future: T[];
}

export type UndoRedoAction<T> =
  | { type: "set"; value: T } // a user edit — pushes present onto past
  | { type: "undo" }
  | { type: "redo" }
  | { type: "reset"; value: T }; // e.g. loading a new Dbc — clears history

export function initUndoRedo<T>(value: T): UndoRedoState<T> {
  return { past: [], present: value, future: [] };
}

const MAX_HISTORY = 100;

export function undoRedoReducer<T>(
  state: UndoRedoState<T>,
  action: UndoRedoAction<T>,
): UndoRedoState<T> {
  switch (action.type) {
    case "set": {
      const past = [...state.past, state.present].slice(-MAX_HISTORY);
      return { past, present: action.value, future: [] };
    }
    case "undo": {
      if (state.past.length === 0) return state;
      const previous = state.past[state.past.length - 1];
      return {
        past: state.past.slice(0, -1),
        present: previous,
        future: [state.present, ...state.future],
      };
    }
    case "redo": {
      if (state.future.length === 0) return state;
      const next = state.future[0];
      return {
        past: [...state.past, state.present],
        present: next,
        future: state.future.slice(1),
      };
    }
    case "reset":
      return initUndoRedo(action.value);
    default:
      return state;
  }
}

export function canUndo<T>(state: UndoRedoState<T>): boolean {
  return state.past.length > 0;
}

export function canRedo<T>(state: UndoRedoState<T>): boolean {
  return state.future.length > 0;
}
