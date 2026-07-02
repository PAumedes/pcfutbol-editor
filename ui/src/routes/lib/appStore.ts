// Svelte-store wiring that glues the pure logic modules (undoRedo,
// validation, autosave, firstRun) to Agent E's ipc.ts. This is the only
// file in ui/src/routes/lib that imports Svelte's store API — everything
// else stays framework-free and is covered by the Vitest suites next to it.
import { derived, get, writable } from "svelte/store";
import type { Dbc } from "../../lib/model";
import { mockDbc } from "../../lib/mocks/dbc";
import * as ipc from "../../lib/ipc";
import {
  canRedo,
  canUndo,
  initUndoRedo,
  undoRedoReducer,
  type UndoRedoState,
} from "./undoRedo";
import { validateDbc, type ValidationError } from "./validation";
import {
  clearAutosave,
  createMemoryStore,
  debounce,
  loadAutosave,
  saveAutosave,
  type KeyValueStore,
} from "./autosave";
import {
  initFirstRun,
  runDetect,
  runLoadPkf,
  runSelectTeam,
  type FirstRunDeps,
  type FirstRunState,
} from "./firstRun";

function browserStore(): KeyValueStore {
  if (typeof localStorage !== "undefined") {
    return localStorage;
  }
  return createMemoryStore();
}

const kvStore = browserStore();

// -- Active Dbc + undo/redo ------------------------------------------------

export const dbcHistory = writable<UndoRedoState<Dbc>>(initUndoRedo(mockDbc));
export const currentDbc = derived(dbcHistory, ($h) => $h.present);
export const validationErrors = derived<typeof currentDbc, ValidationError[]>(
  currentDbc,
  ($dbc) => validateDbc($dbc),
);
export const undoAvailable = derived(dbcHistory, ($h) => canUndo($h));
export const redoAvailable = derived(dbcHistory, ($h) => canRedo($h));
export const gameDir = writable<string | null>(null);

const scheduleAutosave = debounce(() => {
  saveAutosave(kvStore, { dbcs: [get(currentDbc)], gameDir: get(gameDir) });
}, 800);

/** Commits an edit: pushes history and schedules a debounced autosave. */
export function setDbc(next: Dbc): void {
  dbcHistory.update((h) => undoRedoReducer(h, { type: "set", value: next }));
  scheduleAutosave();
}

/** Loads a new Dbc (e.g. after opening a file): clears undo/redo history. */
export function resetDbc(next: Dbc): void {
  dbcHistory.set(initUndoRedo(next));
}

export function undo(): void {
  dbcHistory.update((h) => undoRedoReducer(h, { type: "undo" }));
}

export function redo(): void {
  dbcHistory.update((h) => undoRedoReducer(h, { type: "redo" }));
}

/** Restores the last autosaved project, if any. Returns true if it did. */
export function restoreAutosave(): boolean {
  const envelope = loadAutosave(kvStore);
  if (!envelope || envelope.project.dbcs.length === 0) return false;
  resetDbc(envelope.project.dbcs[0]);
  gameDir.set(envelope.project.gameDir);
  return true;
}

export function discardAutosave(): void {
  clearAutosave(kvStore);
}

// -- First-run flow ---------------------------------------------------------

export const firstRunState = writable<FirstRunState>(initFirstRun());

const firstRunDeps: FirstRunDeps = {
  detectGameDir: ipc.detectGameDir,
  loadPkf: ipc.loadPkf,
  loadPkfTeam: ipc.loadPkfTeam,
};

export async function detectGameFolder(): Promise<void> {
  firstRunState.set({ step: "detecting" });
  firstRunState.set(await runDetect(firstRunDeps));
}

export async function loadGameFolder(dir: string): Promise<void> {
  firstRunState.set({ step: "loading", gameDir: dir });
  const result = await runLoadPkf(firstRunDeps, dir);
  firstRunState.set(result);
  if (result.step === "loaded") {
    gameDir.set(dir);
  }
}

/** Step 3 of first-run: the user picked a team out of the loaded index. */
export async function selectTeam(pointer: number): Promise<void> {
  const state = get(firstRunState);
  if (state.step !== "loaded") return;
  const result = await runSelectTeam(firstRunDeps, state, pointer);
  firstRunState.set(result);
  if (result.step === "team-loaded") {
    resetDbc(result.dbc);
  }
}
