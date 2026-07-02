// Risk #6 (PLAN.md §9): inserting a player mid-list renumbers the team's
// player-pointer block (team k owns pointers (k-1)*50+1..=k*50, assigned by
// list position under "Auto" pointer mode — see §4.2). If photos were
// already imported under the old pointers, they silently desync: pointer 7
// used to be Palermo, now it's someone else's photo.
//
// This module detects when that's about to happen and produces the
// user-facing warning. It does not prevent the insert — squads must stay
// editable — it just makes the risk visible before the user imports photos,
// mirroring the reference editor's guidance ("finish squads before
// importing photos").
import type { PointerMode } from "../../lib/model";

export interface PointerAssignment {
  pointer: number;
  playerId: string; // any stable id (e.g. shortName+birth, or an editor-local uuid)
}

export interface ReshuffleWarning {
  code: "pointer-reshuffle";
  message: string;
  affected: PointerAssignment[];
}

/**
 * Compare the pointer assignment before and after a squad edit (insert,
 * delete, reorder) under Auto pointer mode. Returns a warning listing every
 * pointer whose owning player id changed — those are the photos that would
 * go stale if the user already imported them.
 *
 * Under `PreserveFromFile` mode this always returns no warning, since
 * pointers are never reassigned by position in that mode (§4.2).
 */
export function detectPointerReshuffle(
  before: PointerAssignment[],
  after: PointerAssignment[],
  mode: PointerMode,
): ReshuffleWarning | null {
  if (mode === "preserve_from_file") return null;

  const beforeByPointer = new Map(before.map((a) => [a.pointer, a.playerId]));
  const affected: PointerAssignment[] = [];

  for (const a of after) {
    const previousOwner = beforeByPointer.get(a.pointer);
    if (previousOwner !== undefined && previousOwner !== a.playerId) {
      affected.push(a);
    }
  }

  if (affected.length === 0) return null;

  return {
    code: "pointer-reshuffle",
    affected,
    message:
      `Inserting or reordering players renumbered ${affected.length} pointer(s). ` +
      "Any photos already imported for those pointers now belong to a different player. " +
      "Re-import photos after you finish editing the squad, not before — " +
      "or switch to \"use file's own player pointers\" mode to keep pointers stable.",
  };
}

/**
 * Static banner text for the Crests & Photos screen. Shown unconditionally
 * (not just when a reshuffle is detected) so the user sees the warning
 * before they start importing, per the reference-editor guidance.
 */
export const PHOTO_IMPORT_ORDER_WARNING =
  "Finish editing this team's squad (adding, removing, or reordering players) " +
  "before importing photos. Player pointers are reassigned by position, so " +
  "inserting a player later can silently point an already-imported photo at " +
  "the wrong player.";
