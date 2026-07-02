// First-run flow: "point me at your game folder."
//
// Pure state machine + an orchestration function. The actual IPC calls
// (`detectGameDir`, `loadPkf`) are passed in rather than imported directly,
// so this is testable today against the mocks in ui/src/lib/mocks/dbc.ts
// and swaps to real ui/src/lib/ipc.ts functions with zero logic changes
// once Agent E lands ipc.ts and Agent D lands the backend. Match the exact
// signatures from PLAN.md §4.3:
//   detect_game_dir()      -> Option<String>
//   load_pkf(path: String) -> TeamIndex

import type { TeamIndex } from "../../lib/model";

export type FirstRunState =
  | { step: "idle" }
  | { step: "detecting" }
  | { step: "detected"; gameDir: string }
  | { step: "not-detected" }
  | { step: "loading"; gameDir: string }
  | { step: "loaded"; gameDir: string; teamIndex: TeamIndex }
  | { step: "error"; message: string; gameDir: string | null };

export interface FirstRunDeps {
  detectGameDir: () => Promise<string | null>;
  loadPkf: (path: string) => Promise<TeamIndex>;
}

export function initFirstRun(): FirstRunState {
  return { step: "idle" };
}

/** Step 1: try to auto-detect the game folder. Never throws. */
export async function runDetect(deps: FirstRunDeps): Promise<FirstRunState> {
  try {
    const gameDir = await deps.detectGameDir();
    return gameDir ? { step: "detected", gameDir } : { step: "not-detected" };
  } catch (e) {
    return { step: "error", message: friendlyMessage(e), gameDir: null };
  }
}

/**
 * Step 2: load the team index (EQ003003.PKF) from a folder — either the
 * auto-detected one or one the user picked by hand after "not-detected".
 * Never throws: IO/parse failures become a friendly error state.
 */
export async function runLoadPkf(deps: FirstRunDeps, gameDir: string): Promise<FirstRunState> {
  try {
    const teamIndex = await deps.loadPkf(gameDir);
    if (teamIndex.length === 0) {
      return {
        step: "error",
        gameDir,
        message:
          "That folder doesn't look like a PC Apertura 98/99 install — no teams were found in EQ003003.PKF. Double-check the folder and try again.",
      };
    }
    return { step: "loaded", gameDir, teamIndex };
  } catch (e) {
    return { step: "error", gameDir, message: friendlyMessage(e) };
  }
}

function friendlyMessage(e: unknown): string {
  const raw = e instanceof Error ? e.message : String(e);
  // Never surface a raw stack trace to the user (acceptance bar in
  // PLAN.md §6/Agent F) — map to something actionable, keep the raw detail
  // only for a "details" disclosure the screen may render separately.
  return `We couldn't read that game folder (${raw}). Make sure it points at your PC Apertura 98/99 install and that the files aren't in use by another program.`;
}
