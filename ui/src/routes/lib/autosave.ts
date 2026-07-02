// Autosave for the edit screens. There is no backend persistence yet
// (Agent D/A land later), so this writes to a pluggable key/value store —
// `localStorage` in the browser, an in-memory Map in tests/SSR. The store is
// injected so the logic stays framework-free and unit-testable without a
// DOM.
import type { Project } from "../../lib/model";

export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

/** Drop-in for tests or any environment without `localStorage`. */
export function createMemoryStore(): KeyValueStore {
  const map = new Map<string, string>();
  return {
    getItem: (key) => map.get(key) ?? null,
    setItem: (key, value) => map.set(key, value),
    removeItem: (key) => map.delete(key),
  };
}

export const AUTOSAVE_KEY = "pcf-editor:autosave:project";

export interface AutosaveEnvelope {
  savedAt: string; // ISO timestamp
  project: Project;
}

export function saveAutosave(store: KeyValueStore, project: Project, now = new Date()): void {
  const envelope: AutosaveEnvelope = { savedAt: now.toISOString(), project };
  store.setItem(AUTOSAVE_KEY, JSON.stringify(envelope));
}

/** Returns null if there's nothing saved or the saved payload is corrupt. */
export function loadAutosave(store: KeyValueStore): AutosaveEnvelope | null {
  const raw = store.getItem(AUTOSAVE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as AutosaveEnvelope;
    if (!parsed || typeof parsed.savedAt !== "string" || !parsed.project) return null;
    return parsed;
  } catch {
    return null;
  }
}

export function clearAutosave(store: KeyValueStore): void {
  store.removeItem(AUTOSAVE_KEY);
}

/**
 * Debounce helper: returns a function that only calls `fn` after `delayMs`
 * of quiet. Screens wire this to their undo/redo "set" dispatches so autosave
 * doesn't fire on every keystroke.
 */
export function debounce<Args extends unknown[]>(
  fn: (...args: Args) => void,
  delayMs: number,
): (...args: Args) => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return (...args: Args) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => fn(...args), delayMs);
  };
}
